use chrono::{DateTime, Utc, Duration, Datelike};
use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use super::cash_flow_forecast::{CashFlowForecast, HistoricalTransaction};
use super::liquidity_risk::LiquidityRisk;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecoveryConfidence {
    Strong,     // > 0.75
    Moderate,   // 0.50-0.75
    Weak,       // 0.25-0.50
    None,       // < 0.25
}

impl RecoveryConfidence {
    pub fn from_score(score: f64) -> Self {
        if score > 0.75 { RecoveryConfidence::Strong }
        else if score > 0.50 { RecoveryConfidence::Moderate }
        else if score > 0.25 { RecoveryConfidence::Weak }
        else { RecoveryConfidence::None }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoverySignal {
    pub recovery_confidence: RecoveryConfidence,
    pub recovery_score: f64,
    pub expected_recovery_date: Option<DateTime<Utc>>,
    pub recovery_amount: Decimal,
    pub signals: Vec<RecoveryEvidence>,
    pub is_seasonal_dip: bool,
    pub similar_historical_dips: u32,
    pub recovery_basis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryEvidence {
    pub signal_type: String,
    pub expected_amount: Decimal,
    pub expected_date: Option<DateTime<Utc>>,
    pub confidence: f64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalDip {
    pub start: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
    pub depth: Decimal,
    pub recovered: bool,
    pub recovery_days: Option<u32>,
}

pub struct RecoverySignalDetector;

impl RecoverySignalDetector {
    /// Analysera om en likviditetssvacka förväntas återhämtas
    pub fn detect(
        forecast: &CashFlowForecast,
        risk: &LiquidityRisk,
        history: &[HistoricalTransaction],
    ) -> RecoverySignal {
        let mut signals: Vec<RecoveryEvidence> = Vec::new();
        let mut score_components: Vec<(f64, f64)> = Vec::new();

        // 1. Post-dip recovery in forecast
        if let Some((recovery_date, recovery_amount, confidence)) =
            Self::detect_post_dip_recovery(forecast, risk)
        {
            signals.push(RecoveryEvidence {
                signal_type: "forecast_recovery".to_string(),
                expected_amount: recovery_amount,
                expected_date: Some(recovery_date),
                confidence,
                description: format!(
                    "Balance projected to recover to positive on {}",
                    recovery_date.format("%Y-%m-%d")
                ),
            });
            score_components.push((confidence, 2.0));
        }

        // 2. Recurring inflows during/after dip
        let recurring_evidence = Self::detect_recurring_inflows_in_dip(forecast, risk);
        let recurring_total: Decimal = recurring_evidence.iter().map(|e| e.expected_amount).sum();
        let recurring_score = if recurring_evidence.is_empty() {
            0.0
        } else {
            let avg: f64 = recurring_evidence.iter().map(|e| e.confidence).sum::<f64>()
                / recurring_evidence.len() as f64;
            avg.min(1.0)
        };
        score_components.push((recurring_score, 1.5));
        signals.extend(recurring_evidence);

        // 3. Historical dip analysis
        let historical_dips = Self::find_historical_dips(history);
        let similar_count = historical_dips.iter().filter(|d| d.recovered).count() as u32;
        let historical_score = if historical_dips.is_empty() {
            0.3
        } else {
            let recovered = historical_dips.iter().filter(|d| d.recovered).count();
            recovered as f64 / historical_dips.len() as f64
        };
        score_components.push((historical_score, 1.0));

        if !historical_dips.is_empty() {
            let recovered_count = historical_dips.iter().filter(|d| d.recovered).count().max(1);
            let avg_recovery_days: f64 = historical_dips
                .iter()
                .filter_map(|d| d.recovery_days)
                .map(|d| d as f64)
                .sum::<f64>()
                .max(1.0)
                / recovered_count as f64;

            let avg_depth: Decimal = if historical_dips.is_empty() {
                Decimal::ZERO
            } else {
                let depths: Vec<Decimal> = historical_dips.iter().map(|d| d.depth).collect();
                depths.iter().sum::<Decimal>() / Decimal::from(depths.len() as u64)
            };

            signals.push(RecoveryEvidence {
                signal_type: "historical_pattern".to_string(),
                expected_amount: avg_depth.abs(),
                expected_date: None,
                confidence: historical_score,
                description: format!(
                    "{} of {} similar historical dips recovered (avg {} days)",
                    similar_count,
                    historical_dips.len(),
                    avg_recovery_days.round()
                ),
            });
        }

        // 4. Seasonal score
        let dip_month = risk.dip_start_date
            .map(|d| d.month())
            .unwrap_or_else(|| Utc::now().month());
        let seasonal = Self::seasonal_score(history, dip_month);
        let is_seasonal_dip = seasonal > 0.6;
        score_components.push((seasonal, 0.8));

        if is_seasonal_dip {
            signals.push(RecoveryEvidence {
                signal_type: "seasonal_pattern".to_string(),
                expected_amount: forecast.avg_daily_revenue * Decimal::from(30u64),
                expected_date: None,
                confidence: seasonal,
                description: format!(
                    "Month {} historically shows seasonal cash flow dip with recovery",
                    dip_month
                ),
            });
        }

        // Weighted average recovery score
        let total_weight: f64 = score_components.iter().map(|(_, w)| w).sum();
        let recovery_score = if total_weight > 0.0 {
            score_components.iter().map(|(s, w)| s * w).sum::<f64>() / total_weight
        } else {
            0.0
        };

        let expected_recovery_date = forecast.data_points.iter()
            .find(|p| {
                p.projected_balance > Decimal::ZERO
                    && p.date > risk.dip_start_date.unwrap_or(forecast.generated_at)
            })
            .map(|p| p.date);

        let recovery_amount = if recurring_total > Decimal::ZERO {
            recurring_total
        } else {
            forecast.avg_daily_revenue * Decimal::from(30u64)
        };

        let recovery_basis = if recurring_total > Decimal::ZERO {
            "recurring_revenue".to_string()
        } else if is_seasonal_dip {
            "seasonal_pattern".to_string()
        } else if similar_count > 0 {
            "historical_recovery".to_string()
        } else {
            "estimated_revenue".to_string()
        };

        RecoverySignal {
            recovery_confidence: RecoveryConfidence::from_score(recovery_score),
            recovery_score: recovery_score.clamp(0.0, 1.0),
            expected_recovery_date,
            recovery_amount,
            signals,
            is_seasonal_dip,
            similar_historical_dips: similar_count,
            recovery_basis,
        }
    }

    fn detect_post_dip_recovery(
        forecast: &CashFlowForecast,
        risk: &LiquidityRisk,
    ) -> Option<(DateTime<Utc>, Decimal, f64)> {
        let dip_start = risk.dip_start_date?;
        let mut in_dip = false;
        let mut dip_inflows = Decimal::ZERO;

        for point in &forecast.data_points {
            if point.date >= dip_start && point.projected_balance < Decimal::ZERO {
                in_dip = true;
                dip_inflows += point.expected_inflows;
            } else if in_dip && point.projected_balance >= Decimal::ZERO {
                let confidence = point.confidence * 0.85;
                return Some((point.date, dip_inflows, confidence));
            }
        }
        None
    }

    fn detect_recurring_inflows_in_dip(
        forecast: &CashFlowForecast,
        risk: &LiquidityRisk,
    ) -> Vec<RecoveryEvidence> {
        let mut evidence = Vec::new();
        let window_start = risk.dip_start_date.unwrap_or(forecast.generated_at);
        let window_end = risk.dip_end_date
            .unwrap_or_else(|| window_start + Duration::days(30));

        for point in &forecast.data_points {
            if point.date < window_start || point.date > window_end { continue; }
            if forecast.avg_daily_revenue > Decimal::ZERO
                && point.expected_inflows > forecast.avg_daily_revenue * Decimal::from(2u64)
            {
                let driver = point.drivers.iter()
                    .find(|d| d.starts_with("recurring_"))
                    .cloned()
                    .unwrap_or_else(|| "recurring_inflow".to_string());

                evidence.push(RecoveryEvidence {
                    signal_type: "recurring_revenue".to_string(),
                    expected_amount: point.expected_inflows,
                    expected_date: Some(point.date),
                    confidence: point.confidence * 0.9,
                    description: format!(
                        "Expected inflow of {} from {} on {}",
                        point.expected_inflows,
                        driver,
                        point.date.format("%Y-%m-%d")
                    ),
                });
            }
        }
        evidence
    }

    /// Hitta liknande historiska svackor och deras återhämtning
    pub fn find_historical_dips(history: &[HistoricalTransaction]) -> Vec<HistoricalDip> {
        if history.is_empty() { return Vec::new(); }

        let mut sorted = history.to_vec();
        sorted.sort_by_key(|t| t.date);

        let mut running_balance = Decimal::ZERO;
        let mut daily_balances: Vec<(DateTime<Utc>, Decimal)> = Vec::new();

        for tx in &sorted {
            running_balance += tx.amount;
            if let Some(last) = daily_balances.last_mut() {
                if last.0.date_naive() == tx.date.date_naive() {
                    last.1 = running_balance;
                    continue;
                }
            }
            daily_balances.push((tx.date, running_balance));
        }

        if daily_balances.is_empty() { return Vec::new(); }

        let mean: Decimal = {
            let sum: Decimal = daily_balances.iter().map(|(_, b)| *b).sum();
            sum / Decimal::from(daily_balances.len() as u64)
        };

        let mean_f = mean.to_f64().unwrap_or(0.0);
        let variance_f: f64 = daily_balances.iter()
            .map(|(_, b)| { let d = b.to_f64().unwrap_or(0.0) - mean_f; d * d })
            .sum::<f64>()
            / daily_balances.len() as f64;
        let std_f = variance_f.sqrt();
        let threshold = mean - Decimal::from_f64(std_f * 0.5).unwrap_or(Decimal::ZERO);

        let mut dips: Vec<HistoricalDip> = Vec::new();
        let mut in_dip = false;
        let mut dip_start: Option<DateTime<Utc>> = None;
        let mut dip_depth = Decimal::ZERO;

        for (date, balance) in &daily_balances {
            if *balance < threshold {
                if !in_dip {
                    in_dip = true;
                    dip_start = Some(*date);
                    dip_depth = *balance;
                } else if *balance < dip_depth {
                    dip_depth = *balance;
                }
            } else if in_dip {
                let start = dip_start.unwrap();
                let recovery_days = (*date - start).num_days() as u32;
                dips.push(HistoricalDip {
                    start,
                    end: Some(*date),
                    depth: dip_depth,
                    recovered: true,
                    recovery_days: Some(recovery_days),
                });
                in_dip = false;
                dip_start = None;
                dip_depth = Decimal::ZERO;
            }
        }

        if in_dip {
            if let Some(start) = dip_start {
                dips.push(HistoricalDip {
                    start,
                    end: None,
                    depth: dip_depth,
                    recovered: false,
                    recovery_days: None,
                });
            }
        }

        dips
    }

    /// Beräkna seasonal adjustment
    /// Returnerar 0.0-1.0 där högt = hög sannolikhet att svackan återhämtas (säsongsdriven)
    pub fn seasonal_score(history: &[HistoricalTransaction], dip_month: u32) -> f64 {
        if history.is_empty() { return 0.5; }

        let mut monthly_net: HashMap<u32, Vec<Decimal>> = HashMap::new();
        for tx in history {
            monthly_net.entry(tx.date.month()).or_default().push(tx.amount);
        }

        let mut month_avgs: HashMap<u32, Decimal> = HashMap::new();
        for (month, amounts) in &monthly_net {
            let sum: Decimal = amounts.iter().sum();
            let avg = sum / Decimal::from(amounts.len() as u64);
            month_avgs.insert(*month, avg);
        }

        if month_avgs.len() < 3 { return 0.5; }

        let dip_avg = month_avgs.get(&dip_month).copied().unwrap_or(Decimal::ZERO);
        let all_avgs: Vec<Decimal> = month_avgs.values().copied().collect();
        let months_worse = all_avgs.iter().filter(|&&a| a < dip_avg).count();
        let frac_worse = months_worse as f64 / all_avgs.len() as f64;

        if frac_worse < 0.25 {
            // This is one of the worst months — check if next month recovers
            let next_month = if dip_month == 12 { 1 } else { dip_month + 1 };
            let next_avg = month_avgs.get(&next_month).copied().unwrap_or(Decimal::ZERO);
            if next_avg > dip_avg { 0.75 } else { 0.4 }
        } else {
            0.3 + frac_worse * 0.4
        }
    }
}
