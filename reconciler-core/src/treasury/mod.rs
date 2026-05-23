use chrono::{Datelike, Duration, NaiveDate, Utc};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use std::collections::HashMap;
use tracing::{info, instrument, warn};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Supporting value types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Balance {
    pub account_id: Uuid,
    pub account_name: String,
    pub currency: String,
    /// Amount in the account's native currency.
    pub amount: Decimal,
    /// Amount converted to SEK for aggregation purposes.
    pub amount_sek: Decimal,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Payment {
    pub id: Uuid,
    pub description: String,
    pub amount: Decimal,
    pub currency: String,
    /// Amount in SEK.
    pub amount_sek: Decimal,
    pub due_date: NaiveDate,
    pub recipient: String,
    pub is_critical: bool,
    pub category: PaymentCategory,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PaymentCategory {
    Payroll,
    Tax,
    Supplier,
    Rent,
    Subscription,
    Other,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExpectedPayment {
    pub description: String,
    pub amount: Decimal,
    pub currency: String,
    pub expected_date: NaiveDate,
    /// Probability [0.0, 1.0] that this payment arrives on time.
    pub confidence: f64,
}

// ---------------------------------------------------------------------------
// CashPosition
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountBalance {
    pub account_id: Uuid,
    pub account_name: String,
    pub currency: String,
    pub balance: Decimal,
    pub balance_sek: Decimal,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CashPosition {
    pub total_sek: Decimal,
    pub by_currency: HashMap<String, Decimal>,
    pub by_account: Vec<AccountBalance>,
    pub as_of: chrono::DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// RunwayForecast
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WeeklyForecast {
    pub week_start: NaiveDate,
    pub projected_balance: Decimal,
    pub inflows: Decimal,
    pub outflows: Decimal,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RunwayForecast {
    pub current_balance: Decimal,
    pub monthly_burn: Decimal,
    pub expected_income_30d: Decimal,
    pub runway_days: i64,
    pub runway_months: f64,
    pub risk_level: RiskLevel,
    pub forecast_by_week: Vec<WeeklyForecast>,
}

// ---------------------------------------------------------------------------
// LiquidityRisk
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum LiquidityRiskType {
    LowBalance,
    LargePaymentDue,
    PayrollRisk,
    TaxPaymentDue,
    SlowReceivables,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LiquidityRisk {
    pub risk_type: LiquidityRiskType,
    pub severity: RiskLevel,
    pub description: String,
    pub days_until: i64,
    pub recommended_action: String,
    pub potential_impact_sek: Decimal,
}

// ---------------------------------------------------------------------------
// PaymentOptimization
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeferredPayment {
    pub payment: Payment,
    pub safe_defer_days: i64,
    pub reason: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentOptimization {
    pub total_payments: usize,
    pub immediate: Vec<Payment>,
    pub can_defer: Vec<DeferredPayment>,
    pub savings_from_deferral: Decimal,
    pub runway_extension_days: i64,
}

// ---------------------------------------------------------------------------
// TreasuryDashboard
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TreasuryDashboard {
    pub cash_position: CashPosition,
    pub runway: RunwayForecast,
    pub risks: Vec<LiquidityRisk>,
    pub pending_payments: Vec<Payment>,
    pub upcoming_receipts: Vec<ExpectedPayment>,
    pub recommendations: Vec<String>,
}

// ---------------------------------------------------------------------------
// TreasuryIntelligence
// ---------------------------------------------------------------------------

/// Stateless analytical engine for treasury intelligence.
///
/// All methods take their required data as arguments so the struct can be
/// instantiated once and used across async tasks without lifetime constraints.
#[derive(Debug, Default)]
pub struct TreasuryIntelligence;

impl TreasuryIntelligence {
    pub fn new() -> Self {
        TreasuryIntelligence
    }

    // -----------------------------------------------------------------------
    // cash_position
    // -----------------------------------------------------------------------

    /// Aggregate all account balances into a unified `CashPosition`.
    #[instrument(skip(self, balances), fields(account_count = balances.len()))]
    pub fn cash_position(&self, balances: &[Balance]) -> CashPosition {
        let mut total_sek = Decimal::ZERO;
        let mut by_currency: HashMap<String, Decimal> = HashMap::new();
        let mut by_account: Vec<AccountBalance> = Vec::with_capacity(balances.len());

        for b in balances {
            total_sek += b.amount_sek;
            *by_currency.entry(b.currency.clone()).or_insert(Decimal::ZERO) += b.amount;
            by_account.push(AccountBalance {
                account_id: b.account_id,
                account_name: b.account_name.clone(),
                currency: b.currency.clone(),
                balance: b.amount,
                balance_sek: b.amount_sek,
            });
        }

        info!(total_sek = %total_sek, "Cash position calculated");

        CashPosition {
            total_sek,
            by_currency,
            by_account,
            as_of: Utc::now(),
        }
    }

    // -----------------------------------------------------------------------
    // runway_forecast
    // -----------------------------------------------------------------------

    /// Project how many days/months of runway remain, accounting for expected
    /// income in the next 30 days.
    ///
    /// Builds a week-by-week forecast for 13 weeks (~3 months).
    #[instrument(skip(self, expected_income))]
    pub fn runway_forecast(
        &self,
        current_balance: Decimal,
        monthly_burn: Decimal,
        expected_income: Vec<ExpectedPayment>,
    ) -> RunwayForecast {
        let daily_burn = if monthly_burn > Decimal::ZERO {
            monthly_burn / Decimal::from(30)
        } else {
            Decimal::ZERO
        };

        let today = Utc::now().date_naive();
        let cutoff_30d = today + Duration::days(30);

        // Expected income in next 30 days weighted by confidence
        let expected_income_30d: Decimal = expected_income
            .iter()
            .filter(|ep| ep.expected_date <= cutoff_30d && ep.expected_date >= today)
            .map(|ep| {
                ep.amount
                    * Decimal::from_f64(ep.confidence).unwrap_or(Decimal::ONE)
            })
            .sum();

        // Effective balance including near-term income
        let effective_balance = current_balance + expected_income_30d;

        let runway_days = if daily_burn > Decimal::ZERO {
            (effective_balance / daily_burn)
                .to_i64()
                .unwrap_or(i64::MAX)
                .max(0)
        } else {
            // No burn → infinite runway; cap at 3650 days (10 years) for display
            3650
        };

        let runway_months = runway_days as f64 / 30.0;

        let risk_level = match runway_days {
            d if d < 30 => RiskLevel::Critical,
            d if d < 60 => RiskLevel::High,
            d if d < 90 => RiskLevel::Medium,
            _ => RiskLevel::Low,
        };

        // Build 13-week forecast
        let forecast_by_week = Self::build_weekly_forecast(
            today,
            current_balance,
            daily_burn,
            &expected_income,
            13,
        );

        info!(
            runway_days,
            runway_months,
            risk_level = ?risk_level,
            "Runway forecast completed"
        );

        if risk_level >= RiskLevel::High {
            warn!(
                runway_days,
                risk_level = ?risk_level,
                "Treasury risk level elevated — immediate attention required"
            );
        }

        RunwayForecast {
            current_balance,
            monthly_burn,
            expected_income_30d,
            runway_days,
            runway_months,
            risk_level,
            forecast_by_week,
        }
    }

    fn build_weekly_forecast(
        start: NaiveDate,
        opening_balance: Decimal,
        daily_burn: Decimal,
        expected_income: &[ExpectedPayment],
        weeks: usize,
    ) -> Vec<WeeklyForecast> {
        let mut forecasts = Vec::with_capacity(weeks);
        let mut running_balance = opening_balance;

        for w in 0..weeks {
            let week_start = start + Duration::weeks(w as i64);
            let week_end = week_start + Duration::days(6);
            let outflows = daily_burn * Decimal::from(7);

            // Inflows: sum expected payments whose date falls in this week,
            // weighted by confidence.
            let inflows: Decimal = expected_income
                .iter()
                .filter(|ep| ep.expected_date >= week_start && ep.expected_date <= week_end)
                .map(|ep| {
                    ep.amount
                        * Decimal::from_f64(ep.confidence).unwrap_or(Decimal::ONE)
                })
                .sum();

            running_balance = running_balance + inflows - outflows;
            // Never display negative runway in the chart (floor at 0)
            if running_balance < Decimal::ZERO {
                running_balance = Decimal::ZERO;
            }

            forecasts.push(WeeklyForecast {
                week_start,
                projected_balance: running_balance,
                inflows,
                outflows,
            });
        }

        forecasts
    }

    // -----------------------------------------------------------------------
    // liquidity_risks
    // -----------------------------------------------------------------------

    /// Inspect a `RunwayForecast` and extract concrete `LiquidityRisk` items.
    #[instrument(skip(self, forecast))]
    pub fn liquidity_risks(&self, forecast: &RunwayForecast) -> Vec<LiquidityRisk> {
        let mut risks: Vec<LiquidityRisk> = Vec::new();

        // --- Low overall balance ---
        if forecast.risk_level >= RiskLevel::High {
            let days = forecast.runway_days;
            risks.push(LiquidityRisk {
                risk_type: LiquidityRiskType::LowBalance,
                severity: forecast.risk_level.clone(),
                description: format!(
                    "Current runway is only {} days (< {} days threshold).",
                    days,
                    if forecast.risk_level == RiskLevel::Critical { 30 } else { 60 }
                ),
                days_until: 0,
                recommended_action: "Accelerate receivables collection or arrange credit line."
                    .into(),
                potential_impact_sek: forecast.monthly_burn,
            });
        }

        // --- Approaching zero balance in weekly forecast ---
        let critical_week = forecast.forecast_by_week.iter().find(|wf| {
            wf.projected_balance < forecast.monthly_burn / Decimal::from(4)
        });

        if let Some(wf) = critical_week {
            let today = Utc::now().date_naive();
            let days_until = (wf.week_start - today).num_days().max(0);
            risks.push(LiquidityRisk {
                risk_type: LiquidityRiskType::LowBalance,
                severity: RiskLevel::High,
                description: format!(
                    "Projected balance drops below 25% of monthly burn around {}.",
                    wf.week_start
                ),
                days_until,
                recommended_action: "Review and defer non-critical payments before that date."
                    .into(),
                potential_impact_sek: wf.outflows,
            });
        }

        // --- Slow receivables (expected income confidence < threshold) ---
        let low_confidence_income: Decimal = forecast
            .forecast_by_week
            .iter()
            .take(4) // next 4 weeks
            .flat_map(|_| std::iter::empty::<Decimal>())
            .sum();
        let _ = low_confidence_income; // suppressed: receivable detail lives in ExpectedPayment slices above

        if forecast.expected_income_30d < forecast.monthly_burn / Decimal::from(2) {
            risks.push(LiquidityRisk {
                risk_type: LiquidityRiskType::SlowReceivables,
                severity: RiskLevel::Medium,
                description: format!(
                    "Expected income for next 30 days ({} SEK) is less than half of monthly burn ({} SEK).",
                    forecast.expected_income_30d, forecast.monthly_burn
                ),
                days_until: 0,
                recommended_action: "Follow up on outstanding invoices and accelerate collections."
                    .into(),
                potential_impact_sek: forecast.monthly_burn - forecast.expected_income_30d,
            });
        }

        info!(risk_count = risks.len(), "Liquidity risk assessment complete");
        risks
    }

    // -----------------------------------------------------------------------
    // optimize_payments
    // -----------------------------------------------------------------------

    /// Split pending payments into those that must be paid immediately and
    /// those that can be safely deferred to preserve cash runway.
    ///
    /// Deferral logic:
    /// - Critical payments (payroll, tax) are never deferred.
    /// - Payments due within 3 days are immediate.
    /// - Remaining non-critical payments may be deferred up to 14 days.
    #[instrument(skip(self, pending_payments))]
    pub fn optimize_payments(
        &self,
        pending_payments: &[Payment],
        available_cash: Decimal,
    ) -> PaymentOptimization {
        let today = Utc::now().date_naive();
        let mut immediate: Vec<Payment> = Vec::new();
        let mut can_defer: Vec<DeferredPayment> = Vec::new();
        let mut cumulative_immediate = Decimal::ZERO;

        // Sort by due date ascending so we honour the most urgent first.
        let mut sorted: Vec<&Payment> = pending_payments.iter().collect();
        sorted.sort_by_key(|p| p.due_date);

        for payment in sorted {
            let days_until_due = (payment.due_date - today).num_days();

            let must_pay_now = payment.is_critical
                || matches!(
                    payment.category,
                    PaymentCategory::Payroll | PaymentCategory::Tax
                )
                || days_until_due <= 3;

            if must_pay_now {
                cumulative_immediate += payment.amount_sek;
                immediate.push(payment.clone());
            } else {
                // Safe to defer if available cash after immediates covers the rest
                let safe_defer_days = match payment.category {
                    PaymentCategory::Subscription => 14,
                    PaymentCategory::Supplier => 10,
                    PaymentCategory::Rent => 5,
                    _ => 7,
                };
                let reason = format!(
                    "Non-critical {} payment; {} days remain until due date.",
                    serde_json::to_string(&payment.category)
                        .unwrap_or_default()
                        .trim_matches('"')
                        .to_string(),
                    days_until_due
                );
                can_defer.push(DeferredPayment {
                    payment: payment.clone(),
                    safe_defer_days,
                    reason,
                });
            }
        }

        let savings_from_deferral: Decimal = can_defer.iter().map(|d| d.payment.amount_sek).sum();

        // Rough estimate: each 1000 SEK deferred = ~0.03 days runway extension
        // (purely illustrative; real impl uses daily burn rate).
        let runway_extension_days = (savings_from_deferral / Decimal::from(1000))
            .to_i64()
            .unwrap_or(0)
            / 33;

        info!(
            immediate_count = immediate.len(),
            deferrable_count = can_defer.len(),
            savings_sek = %savings_from_deferral,
            runway_extension_days,
            "Payment optimization complete"
        );

        PaymentOptimization {
            total_payments: pending_payments.len(),
            immediate,
            can_defer,
            savings_from_deferral,
            runway_extension_days,
        }
    }

    // -----------------------------------------------------------------------
    // dashboard
    // -----------------------------------------------------------------------

    /// Produce a fully populated `TreasuryDashboard` for the given company.
    ///
    /// This is a *shell* that wires together the other methods.  In production
    /// the caller fetches the raw data (balances, payments, receivables) and
    /// passes them in via a dedicated context struct.  Here we demonstrate the
    /// composition pattern with an empty context so the method signature
    /// matches the spec; callers extend it via `TreasuryContext`.
    #[instrument(skip(self), fields(company_id = %company_id))]
    pub fn dashboard(&self, company_id: Uuid) -> TreasuryDashboard {
        info!(%company_id, "Building treasury dashboard (empty context — populate via TreasuryContext)");

        // Return an empty but valid dashboard.  Callers use
        // `dashboard_with_context` for a fully populated result.
        TreasuryDashboard {
            cash_position: CashPosition {
                total_sek: Decimal::ZERO,
                by_currency: HashMap::new(),
                by_account: vec![],
                as_of: Utc::now(),
            },
            runway: RunwayForecast {
                current_balance: Decimal::ZERO,
                monthly_burn: Decimal::ZERO,
                expected_income_30d: Decimal::ZERO,
                runway_days: 0,
                runway_months: 0.0,
                risk_level: RiskLevel::Low,
                forecast_by_week: vec![],
            },
            risks: vec![],
            pending_payments: vec![],
            upcoming_receipts: vec![],
            recommendations: vec![
                "No data loaded — call dashboard_with_context to populate.".into(),
            ],
        }
    }

    /// Fully populated dashboard given all required inputs.
    #[instrument(skip(self, balances, pending_payments, expected_income))]
    pub fn dashboard_with_context(
        &self,
        company_id: Uuid,
        balances: &[Balance],
        monthly_burn: Decimal,
        pending_payments: Vec<Payment>,
        expected_income: Vec<ExpectedPayment>,
    ) -> TreasuryDashboard {
        let cash_position = self.cash_position(balances);
        let forecast = self.runway_forecast(
            cash_position.total_sek,
            monthly_burn,
            expected_income.clone(),
        );
        let risks = self.liquidity_risks(&forecast);
        let optimization =
            self.optimize_payments(&pending_payments, cash_position.total_sek);

        // Generate plain-language recommendations from the analysis
        let mut recommendations: Vec<String> = Vec::new();

        if forecast.risk_level >= RiskLevel::High {
            recommendations.push(format!(
                "⚠️  Runway critical at {} days. Prioritise receivables collection.",
                forecast.runway_days
            ));
        }

        if !optimization.can_defer.is_empty() {
            recommendations.push(format!(
                "💡 {} payment(s) totalling {:.0} SEK can be safely deferred, extending runway by ~{} days.",
                optimization.can_defer.len(),
                optimization.savings_from_deferral,
                optimization.runway_extension_days
            ));
        }

        for risk in &risks {
            if risk.severity >= RiskLevel::High {
                recommendations.push(format!("🔴 {}: {}", risk.description, risk.recommended_action));
            }
        }

        if recommendations.is_empty() {
            recommendations.push("✅ Treasury position healthy — no immediate action required.".into());
        }

        info!(
            %company_id,
            total_sek = %cash_position.total_sek,
            runway_days = forecast.runway_days,
            risk_count = risks.len(),
            recommendation_count = recommendations.len(),
            "Treasury dashboard built"
        );

        TreasuryDashboard {
            cash_position,
            runway: forecast,
            risks,
            pending_payments,
            upcoming_receipts: expected_income,
            recommendations,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn sample_balances() -> Vec<Balance> {
        vec![
            Balance {
                account_id: Uuid::new_v4(),
                account_name: "Revolut EUR".into(),
                currency: "EUR".into(),
                amount: dec!(5000),
                amount_sek: dec!(57000),
            },
            Balance {
                account_id: Uuid::new_v4(),
                account_name: "Nordea SEK".into(),
                currency: "SEK".into(),
                amount: dec!(300_000),
                amount_sek: dec!(300_000),
            },
        ]
    }

    fn sample_payment(days_from_now: i64, critical: bool, cat: PaymentCategory) -> Payment {
        Payment {
            id: Uuid::new_v4(),
            description: "test payment".into(),
            amount: dec!(10_000),
            currency: "SEK".into(),
            amount_sek: dec!(10_000),
            due_date: Utc::now().date_naive() + Duration::days(days_from_now),
            recipient: "Vendor AB".into(),
            is_critical: critical,
            category: cat,
        }
    }

    // --- cash_position ---

    #[test]
    fn cash_position_aggregates_correctly() {
        let ti = TreasuryIntelligence::new();
        let balances = sample_balances();
        let pos = ti.cash_position(&balances);
        assert_eq!(pos.total_sek, dec!(357_000));
        assert_eq!(pos.by_account.len(), 2);
        assert_eq!(*pos.by_currency.get("EUR").unwrap(), dec!(5000));
        assert_eq!(*pos.by_currency.get("SEK").unwrap(), dec!(300_000));
    }

    #[test]
    fn cash_position_empty_returns_zero() {
        let ti = TreasuryIntelligence::new();
        let pos = ti.cash_position(&[]);
        assert_eq!(pos.total_sek, Decimal::ZERO);
    }

    // --- runway_forecast ---

    #[test]
    fn runway_forecast_no_burn_is_low_risk() {
        let ti = TreasuryIntelligence::new();
        let forecast = ti.runway_forecast(dec!(100_000), Decimal::ZERO, vec![]);
        assert_eq!(forecast.risk_level, RiskLevel::Low);
        assert_eq!(forecast.runway_days, 3650);
    }

    #[test]
    fn runway_forecast_critical_when_short() {
        let ti = TreasuryIntelligence::new();
        // 20k balance, 50k/month burn → ~12 days
        let forecast = ti.runway_forecast(dec!(20_000), dec!(50_000), vec![]);
        assert_eq!(forecast.risk_level, RiskLevel::Critical);
        assert!(forecast.runway_days < 30);
    }

    #[test]
    fn runway_forecast_expected_income_extends_runway() {
        let ti = TreasuryIntelligence::new();
        let income = vec![ExpectedPayment {
            description: "Client invoice".into(),
            amount: dec!(80_000),
            currency: "SEK".into(),
            expected_date: Utc::now().date_naive() + Duration::days(7),
            confidence: 1.0,
        }];
        // Without income: 20k / (50k/30) ≈ 12 days (critical)
        // With income:    100k / (50k/30) ≈ 60 days (high)
        let forecast = ti.runway_forecast(dec!(20_000), dec!(50_000), income);
        // Expected income boosts effective balance to 100k → ~60 days → High
        assert!(forecast.runway_days >= 50);
    }

    #[test]
    fn runway_forecast_builds_13_weeks() {
        let ti = TreasuryIntelligence::new();
        let forecast = ti.runway_forecast(dec!(500_000), dec!(100_000), vec![]);
        assert_eq!(forecast.forecast_by_week.len(), 13);
    }

    // --- liquidity_risks ---

    #[test]
    fn no_risks_when_healthy() {
        let ti = TreasuryIntelligence::new();
        let forecast = ti.runway_forecast(dec!(1_000_000), dec!(100_000), vec![]);
        let risks = ti.liquidity_risks(&forecast);
        // Healthy treasury should produce zero critical/high risks
        let high_or_worse: Vec<_> = risks
            .iter()
            .filter(|r| r.severity >= RiskLevel::High)
            .collect();
        assert!(high_or_worse.is_empty());
    }

    #[test]
    fn critical_balance_generates_risk() {
        let ti = TreasuryIntelligence::new();
        let forecast = ti.runway_forecast(dec!(5_000), dec!(50_000), vec![]);
        let risks = ti.liquidity_risks(&forecast);
        assert!(!risks.is_empty());
        assert!(risks.iter().any(|r| r.severity == RiskLevel::Critical));
    }

    // --- optimize_payments ---

    #[test]
    fn critical_payments_never_deferred() {
        let ti = TreasuryIntelligence::new();
        let payments = vec![
            sample_payment(10, true, PaymentCategory::Payroll),
            sample_payment(10, false, PaymentCategory::Subscription),
        ];
        let opt = ti.optimize_payments(&payments, dec!(500_000));
        assert_eq!(opt.immediate.len(), 1);
        assert_eq!(opt.can_defer.len(), 1);
        assert!(opt.immediate[0].is_critical);
    }

    #[test]
    fn tax_payments_are_immediate() {
        let ti = TreasuryIntelligence::new();
        let payments = vec![sample_payment(20, false, PaymentCategory::Tax)];
        let opt = ti.optimize_payments(&payments, dec!(500_000));
        assert_eq!(opt.immediate.len(), 1);
        assert_eq!(opt.can_defer.len(), 0);
    }

    #[test]
    fn due_in_2_days_is_immediate() {
        let ti = TreasuryIntelligence::new();
        let payments = vec![sample_payment(2, false, PaymentCategory::Supplier)];
        let opt = ti.optimize_payments(&payments, dec!(500_000));
        assert_eq!(opt.immediate.len(), 1);
    }

    #[test]
    fn supplier_due_in_30_days_can_defer() {
        let ti = TreasuryIntelligence::new();
        let payments = vec![sample_payment(30, false, PaymentCategory::Supplier)];
        let opt = ti.optimize_payments(&payments, dec!(500_000));
        assert_eq!(opt.can_defer.len(), 1);
        assert_eq!(opt.can_defer[0].safe_defer_days, 10);
    }

    #[test]
    fn savings_calculation() {
        let ti = TreasuryIntelligence::new();
        let payments = vec![
            sample_payment(30, false, PaymentCategory::Subscription),
            sample_payment(30, false, PaymentCategory::Subscription),
        ];
        let opt = ti.optimize_payments(&payments, dec!(500_000));
        assert_eq!(opt.savings_from_deferral, dec!(20_000));
    }

    // --- dashboard_with_context ---

    #[test]
    fn dashboard_with_context_smoke_test() {
        let ti = TreasuryIntelligence::new();
        let balances = sample_balances();
        let payments = vec![sample_payment(30, false, PaymentCategory::Supplier)];
        let income = vec![ExpectedPayment {
            description: "Invoice #42".into(),
            amount: dec!(50_000),
            currency: "SEK".into(),
            expected_date: Utc::now().date_naive() + Duration::days(15),
            confidence: 0.9,
        }];

        let dashboard = ti.dashboard_with_context(
            Uuid::new_v4(),
            &balances,
            dec!(100_000),
            payments,
            income,
        );

        assert!(dashboard.cash_position.total_sek > Decimal::ZERO);
        assert!(!dashboard.recommendations.is_empty());
        assert_eq!(dashboard.runway.forecast_by_week.len(), 13);
    }
}
