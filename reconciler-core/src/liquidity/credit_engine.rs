use chrono::{DateTime, Duration, Utc};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::cash_flow_forecast::CashFlowForecast;
use super::liquidity_risk::{LiquidityRisk, RiskLevel};
use super::recovery_signal::{RecoveryConfidence, RecoverySignal};

// ---------------------------------------------------------------------------
// Kreditbeslut
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CreditDecision {
    Approve,       // Erbjud kredit
    Conditional,   // Erbjud med villkor
    Decline,       // Avslå
    Insufficient,  // Inte tillräckligt med data
}

// ---------------------------------------------------------------------------
// DipSummary
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DipSummary {
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
    /// Lägsta projicerade saldo (negativt vid svacka)
    pub projected_minimum: Decimal,
    /// Hur mycket kredit behövs för att täcka svackan + 20% buffer
    pub credit_needed: Decimal,
}

// ---------------------------------------------------------------------------
// CreditOffer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditOffer {
    pub offer_id: Uuid,
    pub generated_at: DateTime<Utc>,
    /// Erbjudandet gäller 72 timmar
    pub expires_at: DateTime<Utc>,
    pub decision: CreditDecision,

    // Kredit-parametrar
    pub credit_amount: Decimal,
    pub currency: String,
    pub credit_duration_days: u32,
    /// T.ex. 0.0008 = 0.08 %/dag
    pub daily_fee_rate: Decimal,
    /// credit_amount * daily_fee_rate * credit_duration_days
    pub total_fee: Decimal,
    /// Effektiv årsränta i procent (compound), t.ex. 33.6
    pub apr: Decimal,
    pub repayment_date: DateTime<Utc>,
    /// Lägsta återbetalningsbelopp: credit_amount + total_fee
    pub min_repayment_amount: Decimal,

    // Analys
    pub liquidity_dip: DipSummary,
    pub recovery_basis: String,
    pub confidence_score: f64,
    pub risk_factors: Vec<String>,
    pub positive_signals: Vec<String>,

    // Text
    pub offer_text_sv: String,
    pub reasoning: String,
}

// ---------------------------------------------------------------------------
// CreditScore
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditScore {
    /// 0–100
    pub score: f64,
    /// "A", "B", "C", "D", "F"
    pub grade: String,
    /// Hur komplett är evidensdatan? 0–1
    pub evidence_quality: f64,
    /// Hur stabila är inbetalningarna? 0–1
    pub revenue_stability: f64,
    /// Hur förutsägbara är kostnaderna? 0–1
    pub burn_predictability: f64,
    /// Sannolikhet att återhämta sig? 0–1
    pub recovery_probability: f64,
    /// Har liknande svackor återhämtats historiskt? 0–1
    pub historical_reliability: f64,
    /// Antal månaders tillgänglig data
    pub data_months: u32,
}

// ---------------------------------------------------------------------------
// CreditEngine
// ---------------------------------------------------------------------------

pub struct CreditEngine;

impl CreditEngine {
    // -----------------------------------------------------------------------
    // evaluate
    // -----------------------------------------------------------------------

    /// Huvud-entry point: tar forecast + risk + recovery → ger ett krediterbjudande.
    pub fn evaluate(
        forecast: &CashFlowForecast,
        risk: &LiquidityRisk,
        recovery: &RecoverySignal,
        data_months_available: u32,
    ) -> CreditOffer {
        let now = Utc::now();
        let offer_id = Uuid::new_v4();
        let expires_at = now + Duration::hours(72);

        // 1. Beräkna credit score
        let score = Self::score(forecast, risk, recovery, data_months_available);

        // 2. Beslutlogik
        let decision = if data_months_available < 2 {
            CreditDecision::Insufficient
        } else if score.score < 50.0 || Self::confidence_is_none(&recovery.recovery_confidence) {
            CreditDecision::Decline
        } else if risk.risk_level == RiskLevel::Insolvent {
            CreditDecision::Decline
        } else if score.score >= 70.0
            && Self::confidence_is_strong_or_moderate(&recovery.recovery_confidence)
        {
            CreditDecision::Approve
        } else if score.score >= 50.0
            && Self::confidence_is_moderate(&recovery.recovery_confidence)
        {
            CreditDecision::Conditional
        } else {
            CreditDecision::Decline
        };

        // 3. Kreditbelopp, duration och avgift
        let credit_amount = Self::calculate_amount(risk);
        let credit_duration_days = Self::calculate_duration(risk, recovery);
        let daily_fee_rate = Self::calculate_fee_rate(&score);

        // 4. Avgifter och APR
        let duration_decimal = Decimal::from(credit_duration_days);
        let total_fee = credit_amount * daily_fee_rate * duration_decimal;
        let min_repayment_amount = credit_amount + total_fee;
        let apr = Self::compute_apr(daily_fee_rate);

        // 5. Återbetalningsdatum
        let repayment_date = now + Duration::days(credit_duration_days as i64);

        // 6. Dip-sammanfattning
        let dip_start = risk.dip_start_date.unwrap_or(now);
        let projected_minimum = risk.dip_depth;
        let shortfall = if risk.dip_depth < Decimal::ZERO {
            risk.dip_depth.abs()
        } else {
            Decimal::ZERO
        };
        let liquidity_dip = DipSummary {
            start_date: dip_start,
            end_date: risk.dip_end_date,
            projected_minimum,
            credit_needed: shortfall * dec!(1.2),
        };

        // 7. Extrahera signaler
        let (positive_signals, risk_factor_strings) =
            Self::extract_signals(recovery, risk);

        // 8. Reasoning
        let reasoning = Self::build_reasoning(
            &decision,
            &score,
            risk,
            recovery,
            credit_amount,
            &forecast.currency,
        );

        // 9. Offer-text
        let offer_text_sv = Self::generate_offer_text_internal(
            &decision,
            credit_amount,
            total_fee,
            daily_fee_rate,
            credit_duration_days,
            risk,
            recovery,
            forecast,
        );

        CreditOffer {
            offer_id,
            generated_at: now,
            expires_at,
            decision,
            credit_amount,
            currency: forecast.currency.clone(),
            credit_duration_days,
            daily_fee_rate,
            total_fee,
            apr,
            repayment_date,
            min_repayment_amount,
            liquidity_dip,
            recovery_basis: recovery.recovery_basis.clone(),
            confidence_score: score.score,
            risk_factors: risk_factor_strings,
            positive_signals,
            offer_text_sv,
            reasoning,
        }
    }

    // -----------------------------------------------------------------------
    // score
    // -----------------------------------------------------------------------

    /// Beräknar en samlad credit score (0–100) med viktade komponenter.
    ///
    /// Vikter: evidence 15% + revenue_stability 25% + burn_predict 20% +
    ///         recovery_prob 30% + historical 10%
    pub fn score(
        forecast: &CashFlowForecast,
        _risk: &LiquidityRisk,
        recovery: &RecoverySignal,
        data_months: u32,
    ) -> CreditScore {
        // Komponent 1: evidence_quality (15%) — modellens konfidens
        let evidence_quality = forecast.model_confidence.clamp(0.0, 1.0);

        // Komponent 2: revenue_stability (25%) — CV av dagliga inflows
        let revenue_stability = Self::compute_revenue_stability(forecast);

        // Komponent 3: burn_predictability (20%) — andel recurring i outflows
        let burn_predictability = Self::compute_burn_predictability(forecast);

        // Komponent 4: recovery_probability (30%) — recovery.recovery_score
        let recovery_probability = recovery.recovery_score.clamp(0.0, 1.0);

        // Komponent 5: historical_reliability (10%) — liknande svackor återhämtade
        let historical_reliability =
            Self::compute_historical_reliability(recovery.similar_historical_dips);

        // Viktad summa
        let weighted = evidence_quality * 0.15
            + revenue_stability * 0.25
            + burn_predictability * 0.20
            + recovery_probability * 0.30
            + historical_reliability * 0.10;

        // Datakvalitets-penalt: < 3 månaders data → 10% avdrag
        let data_penalty = if data_months < 3 { 0.90 } else { 1.0 };
        let raw_score = (weighted * data_penalty * 100.0).clamp(0.0, 100.0);

        let grade = Self::score_to_grade(raw_score);

        CreditScore {
            score: raw_score,
            grade,
            evidence_quality,
            revenue_stability,
            burn_predictability,
            recovery_probability,
            historical_reliability,
            data_months,
        }
    }

    // -----------------------------------------------------------------------
    // calculate_amount
    // -----------------------------------------------------------------------

    /// Bestämmer kreditbelopp: täck svackan + 20% buffer, avrundat till 1 000 kr.
    pub fn calculate_amount(risk: &LiquidityRisk) -> Decimal {
        let shortfall = if risk.dip_depth < Decimal::ZERO {
            risk.dip_depth.abs()
        } else {
            Decimal::ZERO
        };

        if shortfall <= Decimal::ZERO {
            // Minimalt erbjudande om ingen direkt brist detekteras
            dec!(10_000)
        } else {
            let raw = shortfall * dec!(1.2);
            // Avrunda uppåt till närmaste 1 000 kr
            let thousands = (raw / dec!(1_000)).ceil();
            thousands * dec!(1_000)
        }
    }

    // -----------------------------------------------------------------------
    // calculate_duration
    // -----------------------------------------------------------------------

    /// Bestämmer kreditduration: dip_duration_days + 14 dagars grace-period.
    pub fn calculate_duration(risk: &LiquidityRisk, recovery: &RecoverySignal) -> u32 {
        let dip_days = risk.dip_duration_days.unwrap_or(30);
        let base = dip_days + 14;

        // Om recovery-datum är känt, se till att kredit täcker hela perioden
        if let Some(recovery_date) = recovery.expected_recovery_date {
            let now = Utc::now();
            let days_to_recovery = (recovery_date - now).num_days().max(0) as u32;
            let recovery_based = days_to_recovery + 14;
            base.max(recovery_based).min(180) // Max 180 dagar
        } else {
            base.min(180)
        }
    }

    // -----------------------------------------------------------------------
    // calculate_fee_rate
    // -----------------------------------------------------------------------

    /// Beräknar daglig avgiftssats baserat på kreditbetyg.
    ///
    /// Grade A: 0.05%/dag (APR ~19.8%)
    /// Grade B: 0.08%/dag (APR ~33.6%)
    /// Grade C: 0.12%/dag (APR ~55.4%)
    /// Grade D/F: 0 (avslag)
    pub fn calculate_fee_rate(score: &CreditScore) -> Decimal {
        match score.grade.as_str() {
            "A" => dec!(0.0005),
            "B" => dec!(0.0008),
            "C" => dec!(0.0012),
            _ => Decimal::ZERO,
        }
    }

    // -----------------------------------------------------------------------
    // generate_offer_text
    // -----------------------------------------------------------------------

    /// Genererar ett professionellt erbjudandebrev på svenska med företagsnamn.
    pub fn generate_offer_text(offer: &CreditOffer, company_name: &str) -> String {
        if offer.decision == CreditDecision::Decline
            || offer.decision == CreditDecision::Insufficient
        {
            return format!(
                "Hej {},\n\n\
                Vi har analyserat er likviditetssituation och tyvärr kan vi för \
                tillfället inte erbjuda en kredit.\n\n{}\n\n\
                Hör gärna av er om ni har frågor.\n\n\
                Med vänlig hälsning,\n\
                Kvittovalvet Kredit",
                company_name, offer.reasoning
            );
        }

        let dip_start = offer.liquidity_dip.start_date.format("%Y-%m-%d");
        let repayment = offer.repayment_date.format("%Y-%m-%d");
        let expires = offer.expires_at.format("%Y-%m-%d kl. %H:%M");

        let fee_pct = offer.daily_fee_rate * dec!(100);
        let amount_fmt = Self::fmt_amount(offer.credit_amount, &offer.currency);
        let fee_fmt = Self::fmt_amount(offer.total_fee, &offer.currency);
        let repayment_fmt = Self::fmt_amount(offer.min_repayment_amount, &offer.currency);

        let conditions = if offer.decision == CreditDecision::Conditional {
            "\nNotera att detta erbjudande är villkorat och kräver kompletterande \
            verifiering av era intäktsflöden innan utbetalning kan ske.\n"
        } else {
            ""
        };

        format!(
            "Hej {},\n\n\
            Vi har analyserat er likviditetssituation och identifierat en tillfällig \
            likviditetssvacka runt {} på ca {}. Baserat på vår analys av era \
            inbetalningar och historiska mönster bedömer vi att detta är en kortvarig \
            situation med stark återhämtningspotential.\n\n\
            Vi erbjuder er därför en korttidskredit:\n\n\
            • Kreditbelopp: {}\n\
            • Avgift: {} ({:.4}%/dag, effektiv årsränta {:.1}%)\n\
            • Återbetalningstid: {} dagar\n\
            • Återbetalningsdatum: {}\n\
            • Totalt att återbetala: {}\n\
            {}\n\
            Erbjudandet gäller till {}.\n\n\
            Med vänlig hälsning,\n\
            Kvittovalvet Kredit",
            company_name,
            dip_start,
            amount_fmt,
            amount_fmt,
            fee_fmt,
            fee_pct,
            offer.apr,
            offer.credit_duration_days,
            repayment,
            repayment_fmt,
            conditions,
            expires,
        )
    }

    // -----------------------------------------------------------------------
    // Privata hjälpmetoder
    // -----------------------------------------------------------------------

    /// APR = (1 + daily_fee_rate)^365 - 1, uttryckt som procent
    fn compute_apr(daily_fee_rate: Decimal) -> Decimal {
        if daily_fee_rate == Decimal::ZERO {
            return Decimal::ZERO;
        }
        let rate_f64 = daily_fee_rate.to_f64().unwrap_or(0.0);
        let apr_f64 = ((1.0 + rate_f64).powf(365.0) - 1.0) * 100.0;
        Decimal::from_f64_retain(apr_f64)
            .unwrap_or(Decimal::ZERO)
            .round_dp(2)
    }

    fn score_to_grade(score: f64) -> String {
        match score as u32 {
            80..=100 => "A",
            65..=79 => "B",
            50..=64 => "C",
            35..=49 => "D",
            _ => "F",
        }
        .to_string()
    }

    /// Beräknar revenue stability från forecast data points.
    /// Låg CV (stddev/mean) → stabil intäkt → hög score.
    fn compute_revenue_stability(forecast: &CashFlowForecast) -> f64 {
        let inflows: Vec<f64> = forecast
            .data_points
            .iter()
            .filter_map(|p| p.expected_inflows.to_f64())
            .filter(|&v| v >= 0.0)
            .collect();

        if inflows.len() < 2 {
            // Kan inte beräkna CV – base score från avg_daily_revenue vs burn
            if forecast.avg_daily_burn > Decimal::ZERO {
                let ratio = (forecast.avg_daily_revenue / forecast.avg_daily_burn)
                    .to_f64()
                    .unwrap_or(0.5)
                    .min(2.0);
                return (ratio / 2.0).clamp(0.0, 1.0);
            }
            return 0.5;
        }

        let n = inflows.len() as f64;
        let mean = inflows.iter().sum::<f64>() / n;
        if mean <= 0.0 {
            return 0.0;
        }
        let variance = inflows.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
        let stddev = variance.sqrt();
        let cv = stddev / mean;

        // Transformera: låg CV = hög stabilitet
        (1.0 / (1.0 + cv)).clamp(0.0, 1.0)
    }

    /// Beräknar burn predictability: andel outflows som är märkta "recurring".
    fn compute_burn_predictability(forecast: &CashFlowForecast) -> f64 {
        let mut total_outflow = 0.0_f64;
        let mut recurring_outflow = 0.0_f64;

        for point in &forecast.data_points {
            let outflow = point.expected_outflows.to_f64().unwrap_or(0.0);
            total_outflow += outflow;

            // Drivers som innehåller "recurring" anses förutsägbara
            let is_recurring = point
                .drivers
                .iter()
                .any(|d| d.to_lowercase().contains("recurring"));
            if is_recurring {
                recurring_outflow += outflow;
            }
        }

        if total_outflow <= 0.0 {
            // Fallback: använd avg_daily_burn som proxy
            if forecast.avg_daily_burn > Decimal::ZERO {
                return 0.6; // Rimlig default om ingen detaljdata finns
            }
            return 0.5;
        }

        (recurring_outflow / total_outflow).clamp(0.0, 1.0)
    }

    /// Historisk tillförlitlighet baserat på antal liknande svackor som återhämtat sig.
    fn compute_historical_reliability(similar_dips: u32) -> f64 {
        match similar_dips {
            0 => 0.3,  // Inga historiska data → osäker
            1 => 0.6,  // En svacka återhämtad → lovande
            2 => 0.8,  // Två svackor → starkt mönster
            _ => 0.95, // Tre eller fler → mycket starkt mönster
        }
    }

    fn confidence_is_none(c: &RecoveryConfidence) -> bool {
        matches!(c, RecoveryConfidence::None)
    }

    fn confidence_is_strong_or_moderate(c: &RecoveryConfidence) -> bool {
        matches!(
            c,
            RecoveryConfidence::Strong | RecoveryConfidence::Moderate
        )
    }

    fn confidence_is_moderate(c: &RecoveryConfidence) -> bool {
        matches!(c, RecoveryConfidence::Moderate)
    }

    fn extract_signals(
        recovery: &RecoverySignal,
        risk: &LiquidityRisk,
    ) -> (Vec<String>, Vec<String>) {
        let positive: Vec<String> = recovery
            .signals
            .iter()
            .filter(|s| s.confidence > 0.6)
            .map(|s| s.description.clone())
            .collect();

        let risk_strs: Vec<String> = risk
            .risk_factors
            .iter()
            .map(|rf| rf.factor.clone())
            .collect();

        (positive, risk_strs)
    }

    fn build_reasoning(
        decision: &CreditDecision,
        score: &CreditScore,
        risk: &LiquidityRisk,
        recovery: &RecoverySignal,
        credit_amount: Decimal,
        currency: &str,
    ) -> String {
        match decision {
            CreditDecision::Approve => format!(
                "Kredit beviljad. Score: {:.1}/100 (betyg {}). \
                Intäktsstabilitet: {:.0}%, Återhämtningssannolikhet: {:.0}%, \
                Historisk tillförlitlighet: {:.0}%. \
                Svackan bedöms uppgå till {} {} med start {}.",
                score.score,
                score.grade,
                score.revenue_stability * 100.0,
                score.recovery_probability * 100.0,
                score.historical_reliability * 100.0,
                credit_amount,
                currency,
                risk.dip_start_date
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| "okänt datum".to_string()),
            ),
            CreditDecision::Conditional => format!(
                "Villkorad kredit. Score: {:.1}/100 (betyg {}). \
                Måttlig återhämtningskonfidence kräver kompletterande verifiering. \
                Riskfaktorer: {}.",
                score.score,
                score.grade,
                risk.risk_factors
                    .iter()
                    .map(|rf| rf.factor.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            CreditDecision::Decline => format!(
                "Kredit avslås. Score: {:.1}/100 (betyg {}). \
                Otillräcklig återhämtningskonfidence ({:?}) eller för låg score \
                för att motivera ett krediterbjudande.",
                score.score,
                score.grade,
                recovery.recovery_confidence,
            ),
            CreditDecision::Insufficient => format!(
                "Otillräcklig data för kreditbedömning. Tillgängliga månader: {}. \
                Minst 2 månaders data krävs.",
                score.data_months,
            ),
        }
    }

    fn generate_offer_text_internal(
        decision: &CreditDecision,
        credit_amount: Decimal,
        total_fee: Decimal,
        daily_fee_rate: Decimal,
        credit_duration_days: u32,
        risk: &LiquidityRisk,
        recovery: &RecoverySignal,
        forecast: &CashFlowForecast,
    ) -> String {
        if *decision == CreditDecision::Decline || *decision == CreditDecision::Insufficient {
            return "Erbjudandetext ej tillämplig – kredit ej beviljad.".to_string();
        }

        let now = Utc::now();
        let repayment_date = now + Duration::days(credit_duration_days as i64);
        let expires_at = now + Duration::hours(72);

        let dip_start = risk
            .dip_start_date
            .unwrap_or(now)
            .format("%Y-%m-%d")
            .to_string();
        let repayment = repayment_date.format("%Y-%m-%d").to_string();
        let expires = expires_at.format("%Y-%m-%d kl. %H:%M").to_string();

        let fee_pct = daily_fee_rate * dec!(100);
        let apr = Self::compute_apr(daily_fee_rate);
        let min_repayment = credit_amount + total_fee;

        let amount_fmt = Self::fmt_amount(credit_amount, &forecast.currency);
        let fee_fmt = Self::fmt_amount(total_fee, &forecast.currency);
        let repayment_fmt = Self::fmt_amount(min_repayment, &forecast.currency);

        // Bygg positiv-signal-text från recovery evidence
        let positive_signals: Vec<String> = recovery
            .signals
            .iter()
            .filter(|s| s.confidence > 0.6)
            .map(|s| s.description.clone())
            .collect();

        let signals_text = if positive_signals.is_empty() {
            String::new()
        } else {
            format!(
                "Positiva signaler i er verksamhet: {}.\n\n",
                positive_signals.join(", ")
            )
        };

        let conditions = if *decision == CreditDecision::Conditional {
            "\nNotera att detta erbjudande är villkorat och kräver kompletterande \
            verifiering av era intäktsflöden innan utbetalning kan ske.\n"
        } else {
            ""
        };

        format!(
            "Hej,\n\n\
            Vi har analyserat er likviditetssituation och identifierat en tillfällig \
            likviditetssvacka runt {} på ca {}. Baserat på vår analys av era \
            inbetalningar och historiska mönster bedömer vi att detta är en kortvarig \
            situation med stark återhämtningspotential.\n\n\
            {}Vi erbjuder er därför en korttidskredit:\n\n\
            • Kreditbelopp: {}\n\
            • Avgift: {} ({:.4}%/dag, effektiv årsränta {:.1}%)\n\
            • Återbetalningstid: {} dagar\n\
            • Återbetalningsdatum: {}\n\
            • Totalt att återbetala: {}\n\
            {}\n\
            Erbjudandet gäller till {}.\n\n\
            Med vänlig hälsning,\n\
            Kvittovalvet Kredit",
            dip_start,
            amount_fmt,
            signals_text,
            amount_fmt,
            fee_fmt,
            fee_pct,
            apr,
            credit_duration_days,
            repayment,
            repayment_fmt,
            conditions,
            expires,
        )
    }

    /// Formaterar ett belopp med tusentalsavgränsare
    fn fmt_amount(amount: Decimal, currency: &str) -> String {
        let rounded = amount.round_dp(0);
        let s = rounded.to_string();
        let with_sep: String = s
            .chars()
            .rev()
            .enumerate()
            .flat_map(|(i, c)| {
                if i > 0 && i % 3 == 0 {
                    vec![' ', c]
                } else {
                    vec![c]
                }
            })
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        format!("{} {}", with_sep, currency)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::liquidity::cash_flow_forecast::{CashFlowPoint, ForecastWarning};
    use crate::liquidity::liquidity_risk::RiskFactor;
    use crate::liquidity::recovery_signal::RecoveryEvidence;
    use rust_decimal_macros::dec;

    fn sample_forecast() -> CashFlowForecast {
        let now = Utc::now();
        let points: Vec<CashFlowPoint> = (0..60)
            .map(|i| CashFlowPoint {
                date: now + Duration::days(i),
                projected_balance: dec!(150_000) - Decimal::from(i * 2_000),
                expected_inflows: dec!(7_000),
                expected_outflows: dec!(8_000),
                confidence: 0.85,
                drivers: vec!["recurring_salary".to_string(), "recurring_rent".to_string()],
            })
            .collect();

        CashFlowForecast {
            company_id: Some(Uuid::new_v4()),
            generated_at: now,
            current_balance: dec!(150_000),
            currency: "SEK".to_string(),
            horizon_days: 60,
            min_projected_balance: dec!(-30_000),
            min_balance_date: Some(now + Duration::days(45)),
            avg_daily_burn: dec!(8_000),
            avg_daily_revenue: dec!(7_000),
            model_confidence: 0.85,
            warnings: vec![],
            data_points: points,
        }
    }

    fn sample_risk() -> LiquidityRisk {
        LiquidityRisk {
            risk_level: RiskLevel::Warning,
            current_runway_days: Some(18),
            dip_start_date: Some(Utc::now() + Duration::days(15)),
            dip_end_date: Some(Utc::now() + Duration::days(45)),
            dip_depth: dec!(-30_000),
            dip_duration_days: Some(30),
            risk_score: 0.6,
            risk_factors: vec![RiskFactor {
                factor: "projected_negative_balance".to_string(),
                impact: dec!(-30_000),
                date: Some(Utc::now() + Duration::days(15)),
                severity: 0.6,
            }],
        }
    }

    fn sample_recovery() -> RecoverySignal {
        RecoverySignal {
            recovery_confidence: RecoveryConfidence::Strong,
            recovery_score: 0.82,
            expected_recovery_date: Some(Utc::now() + Duration::days(45)),
            recovery_amount: dec!(200_000),
            signals: vec![
                RecoveryEvidence {
                    signal_type: "recurring_revenue".to_string(),
                    expected_amount: dec!(200_000),
                    expected_date: Some(Utc::now() + Duration::days(30)),
                    confidence: 0.85,
                    description: "Återkommande månadsintäkter bekräftade".to_string(),
                },
                RecoveryEvidence {
                    signal_type: "seasonal_pattern".to_string(),
                    expected_amount: dec!(50_000),
                    expected_date: Some(Utc::now() + Duration::days(35)),
                    confidence: 0.70,
                    description: "Historiskt säsongsmönster indikerar återhämtning".to_string(),
                },
            ],
            is_seasonal_dip: false,
            similar_historical_dips: 3,
            recovery_basis: "recurring_revenue".to_string(),
        }
    }

    #[test]
    fn score_produces_valid_range() {
        let forecast = sample_forecast();
        let risk = sample_risk();
        let recovery = sample_recovery();
        let score = CreditEngine::score(&forecast, &risk, &recovery, 3);
        assert!(score.score >= 0.0 && score.score <= 100.0);
        assert!(!score.grade.is_empty());
    }

    #[test]
    fn strong_recovery_yields_approve() {
        let forecast = sample_forecast();
        let risk = sample_risk();
        let recovery = sample_recovery();
        let offer = CreditEngine::evaluate(&forecast, &risk, &recovery, 3);
        assert_eq!(offer.decision, CreditDecision::Approve);
    }

    #[test]
    fn insufficient_data_yields_insufficient() {
        let forecast = sample_forecast();
        let risk = sample_risk();
        let recovery = sample_recovery();
        let offer = CreditEngine::evaluate(&forecast, &risk, &recovery, 1);
        assert_eq!(offer.decision, CreditDecision::Insufficient);
    }

    #[test]
    fn no_recovery_yields_decline() {
        let forecast = sample_forecast();
        let risk = sample_risk();
        let mut recovery = sample_recovery();
        recovery.recovery_confidence = RecoveryConfidence::None;
        recovery.recovery_score = 0.1;
        let offer = CreditEngine::evaluate(&forecast, &risk, &recovery, 3);
        assert_eq!(offer.decision, CreditDecision::Decline);
    }

    #[test]
    fn insolvent_yields_decline() {
        let forecast = sample_forecast();
        let mut risk = sample_risk();
        risk.risk_level = RiskLevel::Insolvent;
        let recovery = sample_recovery();
        let offer = CreditEngine::evaluate(&forecast, &risk, &recovery, 3);
        assert_eq!(offer.decision, CreditDecision::Decline);
    }

    #[test]
    fn credit_amount_includes_buffer() {
        let risk = sample_risk();
        let amount = CreditEngine::calculate_amount(&risk);
        // dip_depth = -30_000, shortfall = 30_000, * 1.2 = 36_000 → rounded to 36_000
        assert!(amount >= dec!(36_000));
    }

    #[test]
    fn duration_includes_grace_period() {
        let risk = sample_risk();
        let recovery = sample_recovery();
        let duration = CreditEngine::calculate_duration(&risk, &recovery);
        // dip_duration_days = 30, + 14 grace = 44; recovery_based = ~45+14 = ~59
        assert!(duration >= 44);
    }

    #[test]
    fn apr_compound_interest_is_correct() {
        // Grade A: 0.05%/dag → APR ≈ 19.8%
        let score = CreditScore {
            score: 82.0,
            grade: "A".to_string(),
            evidence_quality: 0.9,
            revenue_stability: 0.85,
            burn_predictability: 0.8,
            recovery_probability: 0.82,
            historical_reliability: 0.95,
            data_months: 6,
        };
        let rate = CreditEngine::calculate_fee_rate(&score);
        assert_eq!(rate, dec!(0.0005));
        let apr = CreditEngine::compute_apr(rate);
        // (1.0005)^365 - 1 ≈ 0.198 → 19.8%
        assert!(apr > dec!(18.0) && apr < dec!(22.0));
    }

    #[test]
    fn apr_grade_b_is_reasonable() {
        let score = CreditScore {
            score: 70.0,
            grade: "B".to_string(),
            evidence_quality: 0.8,
            revenue_stability: 0.7,
            burn_predictability: 0.7,
            recovery_probability: 0.6,
            historical_reliability: 0.7,
            data_months: 4,
        };
        let rate = CreditEngine::calculate_fee_rate(&score);
        assert_eq!(rate, dec!(0.0008));
        let apr = CreditEngine::compute_apr(rate);
        // (1.0008)^365 - 1 ≈ 0.336 → 33.6%
        assert!(apr > dec!(30.0) && apr < dec!(38.0));
    }

    #[test]
    fn offer_text_contains_key_info() {
        let forecast = sample_forecast();
        let risk = sample_risk();
        let recovery = sample_recovery();
        let offer = CreditEngine::evaluate(&forecast, &risk, &recovery, 3);
        let text = CreditEngine::generate_offer_text(&offer, "Testbolaget AB");
        assert!(text.contains("Testbolaget AB"));
        assert!(text.contains("Kvittovalvet Kredit"));
        assert!(text.contains("72 timmar") || text.contains("gäller till"));
    }

    #[test]
    fn offer_expires_in_72h() {
        let forecast = sample_forecast();
        let risk = sample_risk();
        let recovery = sample_recovery();
        let offer = CreditEngine::evaluate(&forecast, &risk, &recovery, 3);
        let diff = offer.expires_at - offer.generated_at;
        assert_eq!(diff.num_hours(), 72);
    }

    #[test]
    fn min_repayment_equals_principal_plus_fee() {
        let forecast = sample_forecast();
        let risk = sample_risk();
        let recovery = sample_recovery();
        let offer = CreditEngine::evaluate(&forecast, &risk, &recovery, 3);
        assert_eq!(offer.min_repayment_amount, offer.credit_amount + offer.total_fee);
    }

    #[test]
    fn d_grade_yields_zero_fee() {
        let score = CreditScore {
            score: 40.0,
            grade: "D".to_string(),
            evidence_quality: 0.4,
            revenue_stability: 0.3,
            burn_predictability: 0.3,
            recovery_probability: 0.3,
            historical_reliability: 0.3,
            data_months: 2,
        };
        let rate = CreditEngine::calculate_fee_rate(&score);
        assert_eq!(rate, Decimal::ZERO);
    }
}
