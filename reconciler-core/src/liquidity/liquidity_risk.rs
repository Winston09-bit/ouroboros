use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use serde::{Serialize, Deserialize};
use super::cash_flow_forecast::CashFlowForecast;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskLevel {
    Safe,       // Balance > 3x monthly burn, no dips
    Watch,      // Some tension, < 2x monthly burn
    Warning,    // Approaching negative within 30d
    Critical,   // Negative within 14d
    Insolvent,  // Already negative or within 7d
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityRisk {
    pub risk_level: RiskLevel,
    pub current_runway_days: Option<u32>,
    pub dip_start_date: Option<DateTime<Utc>>,
    pub dip_end_date: Option<DateTime<Utc>>,
    pub dip_depth: Decimal,
    pub dip_duration_days: Option<u32>,
    pub risk_score: f64,
    pub risk_factors: Vec<RiskFactor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    pub factor: String,
    pub impact: Decimal,
    pub date: Option<DateTime<Utc>>,
    pub severity: f64,
}

pub struct LiquidityRiskAnalyzer;

impl LiquidityRiskAnalyzer {
    pub fn analyze(forecast: &CashFlowForecast) -> LiquidityRisk {
        let runway = Self::runway_days(forecast);
        let dip_window = Self::find_dip_window(forecast);

        let (dip_start_date, dip_end_date, dip_depth) = match &dip_window {
            Some((start, end, depth)) => (Some(*start), *end, *depth),
            None => (None, None, Decimal::ZERO),
        };

        let dip_duration_days: Option<u32> = match (&dip_start_date, &dip_end_date) {
            (Some(start), Some(end)) => {
                let days = (*end - *start).num_days();
                if days >= 0 { Some(days as u32) } else { None }
            }
            _ => None,
        };

        let monthly_burn = forecast.avg_daily_burn * Decimal::from(30u64);

        let risk_level = if forecast.current_balance < Decimal::ZERO {
            RiskLevel::Insolvent
        } else if let Some(rdays) = runway {
            if rdays <= 7 { RiskLevel::Insolvent }
            else if rdays <= 14 { RiskLevel::Critical }
            else if rdays <= 30 { RiskLevel::Warning }
            else if !monthly_burn.is_zero() && forecast.current_balance < monthly_burn * Decimal::from(2u64) {
                RiskLevel::Watch
            } else { RiskLevel::Safe }
        } else if !monthly_burn.is_zero() && forecast.current_balance < monthly_burn * Decimal::from(3u64) {
            RiskLevel::Watch
        } else {
            RiskLevel::Safe
        };

        let risk_score: f64 = match &risk_level {
            RiskLevel::Safe => {
                if let Some(rdays) = runway { (1.0 - (rdays as f64 / 365.0).min(1.0)) * 0.2 }
                else { 0.05 }
            }
            RiskLevel::Watch => 0.35,
            RiskLevel::Warning => {
                if let Some(rdays) = runway { 0.55 + (1.0 - rdays as f64 / 30.0) * 0.15 }
                else { 0.55 }
            }
            RiskLevel::Critical => {
                if let Some(rdays) = runway { 0.70 + (1.0 - rdays as f64 / 14.0) * 0.15 }
                else { 0.70 }
            }
            RiskLevel::Insolvent => 1.0,
        };

        let mut risk_factors: Vec<RiskFactor> = Vec::new();

        // Large single-day outflows
        for point in &forecast.data_points {
            if forecast.avg_daily_burn > Decimal::ZERO
                && point.expected_outflows > forecast.avg_daily_burn * Decimal::from(3u64)
            {
                risk_factors.push(RiskFactor {
                    factor: "large_recurring_outflow".to_string(),
                    impact: -point.expected_outflows,
                    date: Some(point.date),
                    severity: (point.expected_outflows / (forecast.avg_daily_burn * Decimal::from(3u64)))
                        .to_f64().unwrap_or(1.0).min(1.0),
                });
            }
        }

        // Negative dip
        if dip_depth < Decimal::ZERO {
            risk_factors.push(RiskFactor {
                factor: "projected_negative_balance".to_string(),
                impact: dip_depth,
                date: dip_start_date,
                severity: {
                    let depth_f = dip_depth.abs().to_f64().unwrap_or(0.0);
                    let burn_f = forecast.avg_daily_burn.to_f64().unwrap_or(1.0).max(1.0);
                    (depth_f / (burn_f * 30.0)).min(1.0)
                },
            });
        }

        // Revenue below burn
        if forecast.avg_daily_revenue < forecast.avg_daily_burn && forecast.avg_daily_burn > Decimal::ZERO {
            let gap = forecast.avg_daily_burn - forecast.avg_daily_revenue;
            risk_factors.push(RiskFactor {
                factor: "revenue_below_burn".to_string(),
                impact: -gap,
                date: None,
                severity: (gap / forecast.avg_daily_burn).to_f64().unwrap_or(0.5).min(1.0),
            });
        }

        // Low balance
        let threshold = forecast.avg_daily_burn * Decimal::from(14u64);
        if forecast.current_balance < threshold && forecast.avg_daily_burn > Decimal::ZERO {
            risk_factors.push(RiskFactor {
                factor: "low_current_balance".to_string(),
                impact: forecast.current_balance - threshold,
                date: None,
                severity: {
                    let bal = forecast.current_balance.to_f64().unwrap_or(0.0).max(0.0);
                    let thr = threshold.to_f64().unwrap_or(1.0).max(1.0);
                    1.0 - (bal / thr).min(1.0)
                },
            });
        }

        LiquidityRisk {
            risk_level,
            current_runway_days: runway,
            dip_start_date,
            dip_end_date,
            dip_depth,
            dip_duration_days,
            risk_score: risk_score.clamp(0.0, 1.0),
            risk_factors,
        }
    }

    pub fn runway_days(forecast: &CashFlowForecast) -> Option<u32> {
        if forecast.current_balance < Decimal::ZERO {
            return Some(0);
        }
        for point in &forecast.data_points {
            if point.projected_balance < Decimal::ZERO {
                let days = (point.date - forecast.generated_at).num_days();
                return Some(days.max(0) as u32);
            }
        }
        None
    }

    pub fn find_dip_window(
        forecast: &CashFlowForecast,
    ) -> Option<(DateTime<Utc>, Option<DateTime<Utc>>, Decimal)> {
        let mut best: Option<(DateTime<Utc>, Option<DateTime<Utc>>, Decimal)> = None;
        let mut in_dip = false;
        let mut dip_start: Option<DateTime<Utc>> = None;
        let mut dip_min = Decimal::ZERO;

        for point in &forecast.data_points {
            if point.projected_balance < Decimal::ZERO {
                if !in_dip {
                    in_dip = true;
                    dip_start = Some(point.date);
                    dip_min = point.projected_balance;
                } else if point.projected_balance < dip_min {
                    dip_min = point.projected_balance;
                }
            } else if in_dip {
                let candidate = (dip_start.unwrap(), Some(point.date), dip_min);
                match &best {
                    None => best = Some(candidate),
                    Some((_, _, prev_min)) if dip_min < *prev_min => best = Some(candidate),
                    _ => {}
                }
                in_dip = false;
                dip_start = None;
                dip_min = Decimal::ZERO;
            }
        }

        if in_dip {
            if let Some(start) = dip_start {
                let candidate = (start, None, dip_min);
                match &best {
                    None => best = Some(candidate),
                    Some((_, _, prev_min)) if dip_min < *prev_min => best = Some(candidate),
                    _ => {}
                }
            }
        }

        if best.is_none() {
            if let Some(min_date) = forecast.min_balance_date {
                return Some((min_date, None, forecast.min_projected_balance));
            }
        }

        best
    }
}
