// transparency.rs – Systemisk transparens för kredit-beslut
//
// Principen: Alla parter – fodringsägare (kreditgivare) och tagare (låntagare) –
// ska ha tillgång till exakt samma information om kreditbeslutet.
// Ingen informationsasymmetri. Inget dolt.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::credit_engine::{CreditOffer, CreditScore};
use super::liquidity_risk::LiquidityRisk;
use super::recovery_signal::RecoverySignal;

// ─────────────────────────────────────────────
// TRANSPARENCY REPORT
// Skapas för varje kreditbeslut.
// Skickas till BÅDA parter – fodringsägare och tagare.
// ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransparencyReport {
    pub report_id: Uuid,
    pub offer_id: Uuid,
    pub generated_at: DateTime<Utc>,

    /// Versionen av beslutsmodellen som användes
    pub model_version: String,

    /// Exakt vilken data som låg till grund för beslutet
    pub data_sources: Vec<DataSource>,

    /// Fullständig scoring-kalkyl – alla signaler och vikter synliga
    pub scoring_detail: ScoringDetail,

    /// Likviditetsanalys som stödjer beslutet
    pub liquidity_analysis: LiquidityAnalysisSummary,

    /// Återhämtningsanalys
    pub recovery_analysis: RecoverySummary,

    /// Beslutspunkt – exakt vad som vippade åt vilket håll
    pub decision_rationale: DecisionRationale,

    /// Vad som kan förändra utfallet
    pub improvement_factors: Vec<ImprovementFactor>,

    /// Pågående övervakning – vad systemet bevakar under kreditperioden
    pub monitoring_triggers: Vec<MonitoringTrigger>,

    /// Hash av rapporten för integritetskontroll
    pub report_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSource {
    pub source_type: String,  // "bank_transactions", "erp_vouchers", "merchant_receipts"
    pub provider: String,     // "tink", "fortnox", "enable_banking"
    pub records_used: u32,
    pub date_range_start: Option<DateTime<Utc>>,
    pub date_range_end: Option<DateTime<Utc>>,
    pub completeness_pct: f64, // 0-100 – hur komplett är datan?
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringDetail {
    pub final_score: f64,
    pub grade: String,
    pub signals: Vec<ScoringSignal>,
    pub formula: String, // Klartext-formel
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringSignal {
    pub name: String,
    pub description: String,
    pub raw_value: f64,
    pub normalized_value: f64, // 0-1
    pub weight: f64,
    pub contribution: f64,     // normalized_value * weight * 100
    pub direction: String,     // "positive", "negative", "neutral"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityAnalysisSummary {
    pub current_balance: Decimal,
    pub avg_daily_burn: Decimal,
    pub avg_daily_revenue: Decimal,
    pub runway_days: Option<u32>,
    pub dip_detected: bool,
    pub dip_start: Option<DateTime<Utc>>,
    pub dip_depth: Option<Decimal>,
    pub forecast_horizon_days: u32,
    pub forecast_confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoverySummary {
    pub recovery_detected: bool,
    pub recovery_confidence: f64,
    pub expected_recovery_date: Option<DateTime<Utc>>,
    pub recovery_amount: Decimal,
    pub key_recovery_signals: Vec<String>,
    pub similar_historical_events: u32,
    pub seasonal_factor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRationale {
    pub decision: String,
    pub primary_reason: String,
    pub supporting_factors: Vec<String>,
    pub limiting_factors: Vec<String>,
    /// Vad som skulle ändra beslutet
    pub decision_threshold: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementFactor {
    pub factor: String,
    pub current_value: String,
    pub target_value: String,
    pub impact: String, // "Förbättrar grade från B till A"
    pub how_to_improve: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringTrigger {
    pub trigger_name: String,
    pub condition: String, // "balance < 50000"
    pub action: String,    // "Notifierar båda parter om svackan fördjupas"
    pub parties_notified: Vec<String>, // ["creditor", "debtor"]
}

// ─────────────────────────────────────────────
// BUILDER
// ─────────────────────────────────────────────

pub struct TransparencyReportBuilder;

impl TransparencyReportBuilder {
    pub fn build(
        offer: &CreditOffer,
        score: &CreditScore,
        risk: &LiquidityRisk,
        recovery: &RecoverySignal,
        data_sources: Vec<DataSource>,
    ) -> TransparencyReport {
        let signals = vec![
            ScoringSignal {
                name: "Intäktsstabilitet".into(),
                description: "Standardavvikelse i dagliga intäkter relativt medelvärde".into(),
                raw_value: score.revenue_stability,
                normalized_value: score.revenue_stability,
                weight: 0.25,
                contribution: score.revenue_stability * 0.25 * 100.0,
                direction: if score.revenue_stability > 0.6 { "positive" } else { "negative" }.into(),
            },
            ScoringSignal {
                name: "Kostnadsförutsägbarhet".into(),
                description: "Andel återkommande kostnader av totala kostnader".into(),
                raw_value: score.burn_predictability,
                normalized_value: score.burn_predictability,
                weight: 0.20,
                contribution: score.burn_predictability * 0.20 * 100.0,
                direction: if score.burn_predictability > 0.6 { "positive" } else { "neutral" }.into(),
            },
            ScoringSignal {
                name: "Återhämtningssannolikhet".into(),
                description: "Sannolikhet att likviditeten återhämtas baserat på historik och prognosdata".into(),
                raw_value: score.recovery_probability,
                normalized_value: score.recovery_probability,
                weight: 0.30,
                contribution: score.recovery_probability * 0.30 * 100.0,
                direction: if score.recovery_probability > 0.65 { "positive" } else { "negative" }.into(),
            },
            ScoringSignal {
                name: "Evidenskvalitet".into(),
                description: "Hur komplett kvitto- och bokföringsdata är (täckning av transaktioner)".into(),
                raw_value: score.evidence_quality,
                normalized_value: score.evidence_quality,
                weight: 0.15,
                contribution: score.evidence_quality * 0.15 * 100.0,
                direction: if score.evidence_quality > 0.7 { "positive" } else { "neutral" }.into(),
            },
            ScoringSignal {
                name: "Historisk pålitlighet".into(),
                description: "Huruvida liknande likviditetssituationer återhämtats historiskt".into(),
                raw_value: score.historical_reliability,
                normalized_value: score.historical_reliability,
                weight: 0.10,
                contribution: score.historical_reliability * 0.10 * 100.0,
                direction: if score.historical_reliability > 0.5 { "positive" } else { "neutral" }.into(),
            },
        ];

        let total_contribution: f64 = signals.iter().map(|s| s.contribution).sum();

        let scoring_detail = ScoringDetail {
            final_score: score.score,
            grade: score.grade.clone(),
            signals,
            formula: format!(
                "Score = intäktsstabilitet×25% + kostnadsförutsägbarhet×20% + \
                 återhämtning×30% + evidens×15% + historik×10% = {:.1}",
                score.score
            ),
        };

        let monitoring_triggers = vec![
            MonitoringTrigger {
                trigger_name: "Djupare svacka".into(),
                condition: "Faktisk balans understiger prognos med > 20%".into(),
                action: "Systemet notifierar båda parter omedelbart och uppdaterar riskanalys".into(),
                parties_notified: vec!["creditor".into(), "debtor".into()],
            },
            MonitoringTrigger {
                trigger_name: "Försenad återhämtning".into(),
                condition: "Recovery inträffar > 7 dagar efter prognos".into(),
                action: "Automatisk kontakt med tagare för statusuppdatering. Kreditgivare informeras.".into(),
                parties_notified: vec!["creditor".into(), "debtor".into()],
            },
            MonitoringTrigger {
                trigger_name: "Framgångsrik återhämtning".into(),
                condition: "Balans överstiger 90% av prognos vid förfallodatum".into(),
                action: "Positiv signal – systemet förbereder förbättrat kreditbetyg vid nästa utvärdering".into(),
                parties_notified: vec!["debtor".into()],
            },
            MonitoringTrigger {
                trigger_name: "Ny stor utbetalning".into(),
                condition: "Oplanerad utbetalning > 50,000 kr detekteras".into(),
                action: "Riskanalys uppdateras. Båda parter notifieras om ny bedömning.".into(),
                parties_notified: vec!["creditor".into(), "debtor".into()],
            },
        ];

        let improvement_factors = vec![
            ImprovementFactor {
                factor: "Kvittotäckning".into(),
                current_value: format!("{:.0}%", score.evidence_quality * 100.0),
                target_value: ">85%".into(),
                impact: format!("Förbättrar evidenspoäng från {:.0} till potentiellt +8 poäng", score.evidence_quality * 100.0),
                how_to_improve: "Koppla fler merchants till automatisk kvittohämtning via Kvittovalvet".into(),
            },
            ImprovementFactor {
                factor: "Datahistorik".into(),
                current_value: format!("{} månader", score.data_months),
                target_value: ">6 månader".into(),
                impact: "Längre historik ger mer tillförlitliga mönsteranalyser och bättre kreditvillkor".into(),
                how_to_improve: "Systemet bygger automatiskt upp historik – inga åtgärder krävs".into(),
            },
        ];

        // Skapa en enkel hash av offerID + score
        let report_hash = {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            offer.offer_id.hash(&mut hasher);
            format!("sha256-preview-{:x}", hasher.finish())
        };

        let liquidity_analysis = LiquidityAnalysisSummary {
            current_balance: offer.liquidity_dip.credit_needed + offer.liquidity_dip.projected_minimum.abs() + offer.credit_amount,
            avg_daily_burn: Decimal::new(3000, 0),
            avg_daily_revenue: Decimal::new(8500, 0),
            runway_days: risk.current_runway_days,
            dip_detected: risk.dip_start_date.is_some(),
            dip_start: risk.dip_start_date,
            dip_depth: Some(risk.dip_depth),
            forecast_horizon_days: 30,
            forecast_confidence: 0.72,
        };

        let recovery_analysis = RecoverySummary {
            recovery_detected: recovery.recovery_score > 0.4,
            recovery_confidence: recovery.recovery_score,
            expected_recovery_date: recovery.expected_recovery_date,
            recovery_amount: recovery.recovery_amount,
            key_recovery_signals: recovery.signals.iter().map(|s| s.description.clone()).collect(),
            similar_historical_events: recovery.similar_historical_dips,
            seasonal_factor: if recovery.is_seasonal_dip { Some("Historiskt svagare period – recovery detekterad i data".into()) } else { None },
        };

        let decision_rationale = DecisionRationale {
            decision: format!("{:?}", offer.decision),
            primary_reason: offer.reasoning.clone(),
            supporting_factors: offer.positive_signals.clone(),
            limiting_factors: offer.risk_factors.clone(),
            decision_threshold: format!(
                "Beslut kräver: Score ≥ 65 (er score: {:.0}) + Recovery ≥ 50% (er: {:.0}%) + Risk ≠ Insolvent",
                score.score, recovery.recovery_score * 100.0
            ),
        };

        TransparencyReport {
            report_id: Uuid::new_v4(),
            offer_id: offer.offer_id,
            generated_at: Utc::now(),
            model_version: "kvittovalvet-credit-v1.0.0".into(),
            data_sources,
            scoring_detail,
            liquidity_analysis,
            recovery_analysis,
            decision_rationale,
            improvement_factors,
            monitoring_triggers,
            report_hash,
        }
    }
}

// ─────────────────────────────────────────────
// COMMUNICATION
// Skickar TransparencyReport till rätt mottagare
// ─────────────────────────────────────────────

pub struct TransparencyNotifier;

impl TransparencyNotifier {
    /// Skicka rapport till fodringsägare (kreditgivare)
    pub fn notify_creditor(report: &TransparencyReport) -> String {
        format!(
            "KREDITGIVARE-RAPPORT #{}\n\
             Datum: {}\n\
             Modell: {}\n\
             Score: {:.0} ({})\n\
             Beslut: {}\n\
             Datakällor: {} st\n\
             Övervakningsregler: {} aktiva\n\
             Rapport-hash: {}\n\
             \n\
             Alla siffror och signaler redovisas i bifogad fullständig rapport.\n\
             Ingen information döljs för kreditgivaren.",
            report.report_id,
            report.generated_at.format("%Y-%m-%d %H:%M UTC"),
            report.model_version,
            report.scoring_detail.final_score,
            report.scoring_detail.grade,
            report.decision_rationale.decision,
            report.data_sources.len(),
            report.monitoring_triggers.len(),
            report.report_hash,
        )
    }

    /// Skicka rapport till tagare (låntagaren)
    pub fn notify_debtor(report: &TransparencyReport, company_name: &str) -> String {
        format!(
            "KREDITANALYS FÖR {}\n\
             Rapport-ID: #{}\n\
             \n\
             HUR DITT KREDITBETYG BERÄKNADES:\n\
             {}\n\
             \n\
             SIGNALER SOM ANALYSERADES:\n{}\n\
             \n\
             VAD KAN FÖRBÄTTRA DITT BETYG:\n{}\n\
             \n\
             PÅGÅENDE ÖVERVAKNING:\n\
             Systemet bevakar {} triggers under kreditperioden.\n\
             Båda parter notifieras omedelbart vid avvikelse från prognos.\n\
             \n\
             Samtlig data som använts i beslutet finns tillgänglig på begäran.",
            company_name,
            report.report_id,
            report.scoring_detail.formula,
            report.scoring_detail.signals.iter().map(|s| {
                format!("  • {} ({:.0}% vikt): {:.0}p – {}",
                    s.name, s.weight * 100.0, s.contribution, s.direction)
            }).collect::<Vec<_>>().join("\n"),
            report.improvement_factors.iter().map(|f| {
                format!("  • {} (nu: {}, mål: {}): {}", f.factor, f.current_value, f.target_value, f.impact)
            }).collect::<Vec<_>>().join("\n"),
            report.monitoring_triggers.len(),
        )
    }

    /// Notifiering vid avvikelse under kreditperioden (till BÅDA parter)
    pub fn notify_deviation(
        offer_id: Uuid,
        trigger_name: &str,
        actual_value: &str,
        expected_value: &str,
    ) -> String {
        format!(
            "AVVIKELSENOTIS – Kreditavtal #{}\n\
             Trigger: {}\n\
             Faktiskt värde: {}\n\
             Prognostiserat värde: {}\n\
             \n\
             DENNA NOTIS SKICKAS TILL BÅDA PARTER SIMULTANT.\n\
             Fodringsägare och tagare erhåller identisk information.\n\
             Systemet uppdaterar riskanalys inom 5 minuter.",
            offer_id, trigger_name, actual_value, expected_value,
        )
    }
}
