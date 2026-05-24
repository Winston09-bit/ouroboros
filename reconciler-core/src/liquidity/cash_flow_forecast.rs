use chrono::{DateTime, Utc, Duration, Datelike};
use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use std::collections::HashMap;

/// En historisk transaktion (indata till modellen)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalTransaction {
    pub date: DateTime<Utc>,
    pub amount: Decimal,          // positiv = inbetalning, negativ = utbetalning
    pub currency: String,
    pub category: String,         // "SALARY", "RENT", "DAGLIGVAROR", "REVENUE" etc
    pub counterparty: Option<String>,
    pub is_recurring: bool,
}

/// Ett forecast-datapunkt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashFlowPoint {
    pub date: DateTime<Utc>,
    pub projected_balance: Decimal,
    pub expected_inflows: Decimal,
    pub expected_outflows: Decimal,
    pub confidence: f64,           // 0.0-1.0
    pub drivers: Vec<String>,      // "recurring_salary", "seasonal_revenue" etc
}

/// Resultat av forecasting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashFlowForecast {
    pub company_id: Option<Uuid>,
    pub generated_at: DateTime<Utc>,
    pub current_balance: Decimal,
    pub currency: String,
    pub horizon_days: u32,
    pub data_points: Vec<CashFlowPoint>,
    pub min_projected_balance: Decimal,
    pub min_balance_date: Option<DateTime<Utc>>,
    pub avg_daily_burn: Decimal,
    pub avg_daily_revenue: Decimal,
    pub model_confidence: f64,
    pub warnings: Vec<ForecastWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastWarning {
    pub severity: String,    // "critical", "warning", "info"
    pub date: Option<DateTime<Utc>>,
    pub message: String,
    pub projected_balance: Option<Decimal>,
}

#[derive(Debug, Clone)]
pub struct RecurringPattern {
    pub category: String,
    pub counterparty: Option<String>,
    pub avg_amount: Decimal,
    pub frequency: PatternFrequency,
    pub day_of_month: Option<u32>,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub enum PatternFrequency {
    Daily,
    Weekly,
    BiWeekly,
    Monthly,
    Quarterly,
    Irregular { avg_days_between: f64 },
}

pub struct CashFlowForecaster;

impl CashFlowForecaster {
    /// Analysera historik och identifiera återkommande mönster
    pub fn detect_patterns(transactions: &[HistoricalTransaction]) -> Vec<RecurringPattern> {
        let mut groups: HashMap<(String, String), Vec<&HistoricalTransaction>> = HashMap::new();
        for tx in transactions {
            let key = (
                tx.counterparty.clone().unwrap_or_default(),
                tx.category.clone(),
            );
            groups.entry(key).or_default().push(tx);
        }

        let mut patterns = Vec::new();

        for ((counterparty_str, category), mut group) in groups {
            if group.len() < 2 {
                continue;
            }
            group.sort_by_key(|t| t.date);

            let mut amounts: Vec<Decimal> = group.iter().map(|t| t.amount.abs()).collect();
            amounts.sort();
            let median_amount = amounts[amounts.len() / 2];
            if median_amount.is_zero() {
                continue;
            }

            let sum: Decimal = amounts.iter().sum();
            let count = Decimal::from(amounts.len() as u64);
            let mean = sum / count;

            let variance: Decimal = amounts
                .iter()
                .map(|a| { let d = *a - mean; d * d })
                .sum::<Decimal>()
                / count;

            let stddev = {
                let v: f64 = variance.to_f64().unwrap_or(0.0);
                Decimal::from_f64(v.sqrt()).unwrap_or(Decimal::ZERO)
            };

            let cv = if !mean.is_zero() {
                (stddev / mean).to_f64().unwrap_or(1.0)
            } else { 1.0 };

            if cv > 0.30 { continue; }

            let mut intervals: Vec<f64> = Vec::new();
            for i in 1..group.len() {
                let diff = (group[i].date - group[i - 1].date).num_days() as f64;
                if diff > 0.0 { intervals.push(diff); }
            }
            if intervals.is_empty() { continue; }

            let avg_interval = intervals.iter().sum::<f64>() / intervals.len() as f64;
            let interval_var: f64 = intervals
                .iter()
                .map(|i| (i - avg_interval).powi(2))
                .sum::<f64>()
                / intervals.len() as f64;
            let interval_cv = if avg_interval > 0.0 { interval_var.sqrt() / avg_interval } else { 1.0 };

            let confidence = ((1.0 - cv) * 0.5 + (1.0 - interval_cv).max(0.0) * 0.5).min(1.0);
            if confidence < 0.3 { continue; }

            let frequency = if avg_interval < 2.0 {
                PatternFrequency::Daily
            } else if (avg_interval - 7.0).abs() < 2.0 {
                PatternFrequency::Weekly
            } else if (avg_interval - 14.0).abs() < 3.0 {
                PatternFrequency::BiWeekly
            } else if (avg_interval - 30.0).abs() < 5.0 {
                PatternFrequency::Monthly
            } else if (avg_interval - 91.0).abs() < 10.0 {
                PatternFrequency::Quarterly
            } else {
                PatternFrequency::Irregular { avg_days_between: avg_interval }
            };

            let day_of_month = if matches!(frequency, PatternFrequency::Monthly) {
                let s: u32 = group.iter().map(|t| t.date.day()).sum();
                Some(s / group.len() as u32)
            } else { None };

            let signed_sum: Decimal = group.iter().map(|t| t.amount).sum();
            let avg_amount = signed_sum / Decimal::from(group.len() as u64);

            let counterparty = if counterparty_str.is_empty() { None } else { Some(counterparty_str) };

            patterns.push(RecurringPattern {
                category,
                counterparty,
                avg_amount,
                frequency,
                day_of_month,
                confidence,
            });
        }
        patterns
    }

    /// Generera forecast för N dagar framåt
    pub fn forecast(
        current_balance: Decimal,
        currency: String,
        history: &[HistoricalTransaction],
        horizon_days: u32,
    ) -> CashFlowForecast {
        let patterns = Self::detect_patterns(history);
        let now = Utc::now();
        let ninety_ago = now - Duration::days(90);

        let recent: Vec<&HistoricalTransaction> = history
            .iter()
            .filter(|t| t.date >= ninety_ago)
            .collect();

        let total_inflows: Decimal = recent.iter().filter(|t| t.amount > Decimal::ZERO).map(|t| t.amount).sum();
        let total_outflows: Decimal = recent.iter().filter(|t| t.amount < Decimal::ZERO).map(|t| t.amount.abs()).sum();
        let days90 = Decimal::from(90u64);
        let avg_daily_revenue = total_inflows / days90.max(Decimal::ONE);
        let avg_daily_burn = total_outflows / days90.max(Decimal::ONE);

        struct PatternState {
            next_fire: DateTime<Utc>,
            interval_days: f64,
            amount: Decimal,
            category: String,
        }

        let mut pattern_states: Vec<PatternState> = patterns.iter().map(|p| {
            let interval_days = match &p.frequency {
                PatternFrequency::Daily => 1.0,
                PatternFrequency::Weekly => 7.0,
                PatternFrequency::BiWeekly => 14.0,
                PatternFrequency::Monthly => 30.0,
                PatternFrequency::Quarterly => 91.0,
                PatternFrequency::Irregular { avg_days_between } => *avg_days_between,
            };
            let last_match = history.iter()
                .filter(|t| t.category == p.category && t.counterparty == p.counterparty)
                .max_by_key(|t| t.date);
            let next_fire = if let Some(last) = last_match {
                last.date + Duration::days(interval_days.round() as i64)
            } else {
                now + Duration::days(interval_days.round() as i64)
            };
            PatternState { next_fire, interval_days, amount: p.avg_amount, category: p.category.clone() }
        }).collect();

        let mut data_points = Vec::with_capacity(horizon_days as usize);
        let mut balance = current_balance;
        let mut min_balance = current_balance;
        let mut min_balance_date: Option<DateTime<Utc>> = None;
        let mut warnings: Vec<ForecastWarning> = Vec::new();
        let warn_50k = Decimal::from(50_000u64);
        let warn_100k = Decimal::from(100_000u64);

        for day_offset in 1..=horizon_days {
            let day_date = now + Duration::days(day_offset as i64);
            let confidence = (0.95 - 0.02 * (day_offset as f64 - 1.0)).max(0.05);
            let mut day_inflows = Decimal::ZERO;
            let mut day_outflows = Decimal::ZERO;
            let mut drivers: Vec<String> = Vec::new();

            for state in pattern_states.iter_mut() {
                while state.next_fire.date_naive() <= day_date.date_naive() {
                    if state.amount > Decimal::ZERO {
                        day_inflows += state.amount;
                    } else {
                        day_outflows += state.amount.abs();
                    }
                    drivers.push(format!("recurring_{}", state.category.to_lowercase()));
                    state.next_fire = state.next_fire + Duration::days(state.interval_days.round() as i64);
                }
            }

            let weekday = day_date.weekday();
            let is_weekday = !matches!(weekday, chrono::Weekday::Sat | chrono::Weekday::Sun);
            if is_weekday && avg_daily_revenue > Decimal::ZERO {
                let wd_revenue = avg_daily_revenue * Decimal::from(7u64) / Decimal::from(5u64);
                day_inflows += wd_revenue;
                drivers.push("estimated_daily_revenue".to_string());
            }

            if day_outflows < avg_daily_burn {
                let residual = avg_daily_burn - day_outflows;
                if residual > Decimal::ZERO {
                    day_outflows += residual;
                    drivers.push("baseline_burn".to_string());
                }
            }

            balance += day_inflows;
            balance -= day_outflows;

            if balance < min_balance {
                min_balance = balance;
                min_balance_date = Some(day_date);
            }

            if balance < Decimal::ZERO {
                warnings.push(ForecastWarning {
                    severity: "critical".to_string(),
                    date: Some(day_date),
                    message: format!("Projected negative balance on {}", day_date.format("%Y-%m-%d")),
                    projected_balance: Some(balance),
                });
            } else if balance < warn_50k {
                warnings.push(ForecastWarning {
                    severity: "warning".to_string(),
                    date: Some(day_date),
                    message: format!("Balance below 50 000 on {}", day_date.format("%Y-%m-%d")),
                    projected_balance: Some(balance),
                });
            } else if balance < warn_100k {
                warnings.push(ForecastWarning {
                    severity: "info".to_string(),
                    date: Some(day_date),
                    message: format!("Balance below 100 000 on {}", day_date.format("%Y-%m-%d")),
                    projected_balance: Some(balance),
                });
            }

            drivers.dedup();
            data_points.push(CashFlowPoint {
                date: day_date,
                projected_balance: balance,
                expected_inflows: day_inflows,
                expected_outflows: day_outflows,
                confidence,
                drivers,
            });
        }

        warnings.dedup_by(|a, b| a.severity == b.severity);

        let model_confidence = if data_points.is_empty() { 0.95 } else {
            data_points.iter().map(|p| p.confidence).sum::<f64>() / data_points.len() as f64
        };

        CashFlowForecast {
            company_id: None,
            generated_at: now,
            current_balance,
            currency,
            horizon_days,
            data_points,
            min_projected_balance: min_balance,
            min_balance_date,
            avg_daily_burn,
            avg_daily_revenue,
            model_confidence,
            warnings,
        }
    }

    /// Snabb forecast baserat på enbart burn rate
    pub fn burn_rate_forecast(
        current_balance: Decimal,
        daily_burn: Decimal,
        daily_revenue: Decimal,
        horizon_days: u32,
    ) -> CashFlowForecast {
        let now = Utc::now();
        let net_daily = daily_revenue - daily_burn;
        let mut data_points = Vec::with_capacity(horizon_days as usize);
        let mut balance = current_balance;
        let mut min_balance = current_balance;
        let mut min_balance_date: Option<DateTime<Utc>> = None;
        let mut warnings: Vec<ForecastWarning> = Vec::new();
        let warn_50k = Decimal::from(50_000u64);
        let warn_100k = Decimal::from(100_000u64);

        for day_offset in 1..=horizon_days {
            let day_date = now + Duration::days(day_offset as i64);
            let confidence = (0.95 - 0.02 * (day_offset as f64 - 1.0)).max(0.05);
            balance += net_daily;

            if balance < min_balance {
                min_balance = balance;
                min_balance_date = Some(day_date);
            }

            if balance < Decimal::ZERO {
                warnings.push(ForecastWarning {
                    severity: "critical".to_string(),
                    date: Some(day_date),
                    message: format!("Projected negative balance on {}", day_date.format("%Y-%m-%d")),
                    projected_balance: Some(balance),
                });
            } else if balance < warn_50k {
                warnings.push(ForecastWarning {
                    severity: "warning".to_string(),
                    date: Some(day_date),
                    message: format!("Balance below 50 000 on {}", day_date.format("%Y-%m-%d")),
                    projected_balance: Some(balance),
                });
            } else if balance < warn_100k {
                warnings.push(ForecastWarning {
                    severity: "info".to_string(),
                    date: Some(day_date),
                    message: format!("Balance below 100 000 on {}", day_date.format("%Y-%m-%d")),
                    projected_balance: Some(balance),
                });
            }

            data_points.push(CashFlowPoint {
                date: day_date,
                projected_balance: balance,
                expected_inflows: daily_revenue,
                expected_outflows: daily_burn,
                confidence,
                drivers: vec!["burn_rate_model".to_string()],
            });
        }

        warnings.dedup_by(|a, b| a.severity == b.severity);

        let model_confidence = if data_points.is_empty() { 0.95 } else {
            data_points.iter().map(|p| p.confidence).sum::<f64>() / data_points.len() as f64
        };

        CashFlowForecast {
            company_id: None,
            generated_at: now,
            current_balance,
            currency: "SEK".to_string(),
            horizon_days,
            data_points,
            min_projected_balance: min_balance,
            min_balance_date,
            avg_daily_burn: daily_burn,
            avg_daily_revenue: daily_revenue,
            model_confidence,
            warnings,
        }
    }
}
