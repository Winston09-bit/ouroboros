// src/sandbox/bank_simulator.rs
//
// Simulates a real-time bank environment: three accounts (Nordea, Revolut, SEB),
// 30 merchant templates, randomised daily transactions, webhook delivery,
// and chaos helpers (duplicate injection, webhook failure simulation).

use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use uuid::Uuid;

// ─────────────────────────────────────────────────────────────────────────────
// Transaction status
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TxStatus {
    Completed,
    Pending,
    Failed,
    Refunded,
}

impl std::fmt::Display for TxStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TxStatus::Completed => write!(f, "completed"),
            TxStatus::Pending   => write!(f, "pending"),
            TxStatus::Failed    => write!(f, "failed"),
            TxStatus::Refunded  => write!(f, "refunded"),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Merchant catalog  (30 entries; realistic Swedish + international mix)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct MerchantTemplate {
    pub name:       &'static str,
    pub category:   &'static str,
    pub min_amount: f64,
    pub max_amount: f64,
    pub currency:   &'static str,
    pub vat_rate:   f64,   // 0.06, 0.12, or 0.25
}

pub const MERCHANT_CATALOG: &[MerchantTemplate] = &[
    MerchantTemplate { name: "ICA Maxi Sundbyberg",   category: "Groceries",   min_amount: 150.0,  max_amount: 2800.0,  currency: "SEK", vat_rate: 0.12 },
    MerchantTemplate { name: "Coop Konsum",            category: "Groceries",   min_amount: 80.0,   max_amount: 1500.0,  currency: "SEK", vat_rate: 0.12 },
    MerchantTemplate { name: "Lidl Sverige AB",        category: "Groceries",   min_amount: 50.0,   max_amount: 900.0,   currency: "SEK", vat_rate: 0.12 },
    MerchantTemplate { name: "Hemköp",                 category: "Groceries",   min_amount: 100.0,  max_amount: 1200.0,  currency: "SEK", vat_rate: 0.12 },
    MerchantTemplate { name: "Telia Sverige AB",       category: "Telecom",     min_amount: 199.0,  max_amount: 799.0,   currency: "SEK", vat_rate: 0.25 },
    MerchantTemplate { name: "Telenor Sverige AB",     category: "Telecom",     min_amount: 299.0,  max_amount: 699.0,   currency: "SEK", vat_rate: 0.25 },
    MerchantTemplate { name: "Tre Sverige AB",         category: "Telecom",     min_amount: 179.0,  max_amount: 549.0,   currency: "SEK", vat_rate: 0.25 },
    MerchantTemplate { name: "Amazon Web Services",    category: "Cloud",       min_amount: 50.0,   max_amount: 8000.0,  currency: "EUR", vat_rate: 0.25 },
    MerchantTemplate { name: "Google Cloud Platform",  category: "Cloud",       min_amount: 20.0,   max_amount: 5000.0,  currency: "EUR", vat_rate: 0.25 },
    MerchantTemplate { name: "Microsoft Azure",        category: "Cloud",       min_amount: 100.0,  max_amount: 6000.0,  currency: "EUR", vat_rate: 0.25 },
    MerchantTemplate { name: "Spotify AB",             category: "Software",    min_amount: 99.0,   max_amount: 299.0,   currency: "SEK", vat_rate: 0.25 },
    MerchantTemplate { name: "Netflix International",  category: "Software",    min_amount: 119.0,  max_amount: 179.0,   currency: "SEK", vat_rate: 0.25 },
    MerchantTemplate { name: "Adobe Systems Inc",      category: "Software",    min_amount: 349.0,  max_amount: 599.0,   currency: "SEK", vat_rate: 0.25 },
    MerchantTemplate { name: "Slack Technologies LLC", category: "Software",    min_amount: 70.0,   max_amount: 800.0,   currency: "EUR", vat_rate: 0.25 },
    MerchantTemplate { name: "GitHub Inc",             category: "Software",    min_amount: 40.0,   max_amount: 200.0,   currency: "USD", vat_rate: 0.25 },
    MerchantTemplate { name: "Uber Technologies",      category: "Transport",   min_amount: 60.0,   max_amount: 450.0,   currency: "SEK", vat_rate: 0.06 },
    MerchantTemplate { name: "Bolt Operations OÜ",    category: "Transport",   min_amount: 45.0,   max_amount: 350.0,   currency: "SEK", vat_rate: 0.06 },
    MerchantTemplate { name: "SJ AB",                  category: "Transport",   min_amount: 199.0,  max_amount: 2200.0,  currency: "SEK", vat_rate: 0.06 },
    MerchantTemplate { name: "FlixBus GmbH",           category: "Transport",   min_amount: 99.0,   max_amount: 499.0,   currency: "SEK", vat_rate: 0.06 },
    MerchantTemplate { name: "IKEA Sverige AB",        category: "Furniture",   min_amount: 200.0,  max_amount: 8000.0,  currency: "SEK", vat_rate: 0.25 },
    MerchantTemplate { name: "H&M Hennes & Mauritz",   category: "Clothing",    min_amount: 150.0,  max_amount: 1500.0,  currency: "SEK", vat_rate: 0.25 },
    MerchantTemplate { name: "Systembolaget AB",       category: "Beverages",   min_amount: 89.0,   max_amount: 900.0,   currency: "SEK", vat_rate: 0.25 },
    MerchantTemplate { name: "Apoteket AB",            category: "Pharmacy",    min_amount: 50.0,   max_amount: 600.0,   currency: "SEK", vat_rate: 0.25 },
    MerchantTemplate { name: "McDonald's Sverige",     category: "Restaurant",  min_amount: 60.0,   max_amount: 280.0,   currency: "SEK", vat_rate: 0.12 },
    MerchantTemplate { name: "Max Hamburgare AB",      category: "Restaurant",  min_amount: 75.0,   max_amount: 320.0,   currency: "SEK", vat_rate: 0.12 },
    MerchantTemplate { name: "PostNord Sverige AB",    category: "Logistics",   min_amount: 45.0,   max_amount: 800.0,   currency: "SEK", vat_rate: 0.25 },
    MerchantTemplate { name: "DHL Express Sweden",     category: "Logistics",   min_amount: 89.0,   max_amount: 1200.0,  currency: "SEK", vat_rate: 0.25 },
    MerchantTemplate { name: "Klarna Bank AB",         category: "Financial",   min_amount: 100.0,  max_amount: 5000.0,  currency: "SEK", vat_rate: 0.25 },
    MerchantTemplate { name: "Fortnox AB",             category: "Software",    min_amount: 449.0,  max_amount: 1299.0,  currency: "SEK", vat_rate: 0.25 },
    MerchantTemplate { name: "Volvo Cars AB",          category: "Automotive",  min_amount: 500.0,  max_amount: 15000.0, currency: "SEK", vat_rate: 0.25 },
];

// ─────────────────────────────────────────────────────────────────────────────
// Core data types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedTransaction {
    pub id:             Uuid,
    pub amount:         Decimal,
    pub currency:       String,
    pub merchant:       String,
    pub category:       String,
    pub vat_rate:       Decimal,
    pub vat_amount:     Decimal,
    pub net_amount:     Decimal,
    pub timestamp:      DateTime<Utc>,
    pub status:         TxStatus,
    pub receipt_exists: bool,
    pub account_id:     String,
    pub reference:      String,
    pub description:    String,
    pub is_duplicate:   bool,
    pub webhook_fired:  bool,
}

impl SimulatedTransaction {
    /// Build from merchant template; caller supplies account_id, amount, and timestamp.
    fn new(
        account_id: impl Into<String>,
        merchant: &MerchantTemplate,
        amount: Decimal,
        timestamp: DateTime<Utc>,
        receipt_exists: bool,
    ) -> Self {
        let vat_rate   = Decimal::from_f64(merchant.vat_rate).unwrap_or_default();
        let net_amount = (amount / (Decimal::ONE + vat_rate)).round_dp(2);
        let vat_amount = (amount - net_amount).round_dp(2);

        Self {
            id:             Uuid::new_v4(),
            amount,
            currency:       merchant.currency.to_string(),
            merchant:       merchant.name.to_string(),
            category:       merchant.category.to_string(),
            vat_rate,
            vat_amount,
            net_amount,
            timestamp,
            status:         TxStatus::Completed,
            receipt_exists,
            account_id:     account_id.into(),
            reference:      format!("REF-{}", &Uuid::new_v4().to_string()[..8].to_uppercase()),
            description:    format!("Purchase at {}", merchant.name),
            is_duplicate:   false,
            webhook_fired:  false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedAccount {
    pub id:           String,
    pub owner:        String,
    pub balance:      Decimal,
    pub currency:     String,
    pub bank_name:    String,
    pub iban:         String,
    pub transactions: Vec<SimulatedTransaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountSummary {
    pub id:                String,
    pub owner:             String,
    pub bank_name:         String,
    pub balance:           Decimal,
    pub currency:          String,
    pub transaction_count: usize,
    pub total_debited:     Decimal,
    pub total_vat:         Decimal,
    pub pending_count:     usize,
    pub missing_receipts:  usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub event:       String,
    pub transaction: SimulatedTransaction,
    pub account_id:  String,
    pub timestamp:   DateTime<Utc>,
    pub retry:       u32,
}

// ─────────────────────────────────────────────────────────────────────────────
// BankSimulator
// ─────────────────────────────────────────────────────────────────────────────

pub struct BankSimulator {
    pub accounts:                 HashMap<String, SimulatedAccount>,
    pub webhook_url:              Option<String>,
    pub failed_webhooks:          Vec<SimulatedTransaction>,
    pub duplicate_pairs:          Vec<(SimulatedTransaction, SimulatedTransaction)>,
    simulate_webhook_failures:    bool,
    request_count:                u64,
}

impl Default for BankSimulator {
    fn default() -> Self { Self::new() }
}

impl BankSimulator {
    // ── Construction ────────────────────────────────────────────────────────

    pub fn new() -> Self {
        let mut sim = Self {
            accounts:                  HashMap::new(),
            webhook_url:               None,
            failed_webhooks:           Vec::new(),
            duplicate_pairs:           Vec::new(),
            simulate_webhook_failures: false,
            request_count:             0,
        };
        sim.initialize_accounts();
        sim
    }

    pub fn with_webhook(mut self, url: impl Into<String>) -> Self {
        self.webhook_url = Some(url.into());
        self
    }

    fn initialize_accounts(&mut self) {
        let accounts = vec![
            SimulatedAccount {
                id:           "acc-nordea-001".to_string(),
                owner:        "LandveX AB".to_string(),
                balance:      Decimal::from_str("485320.50").unwrap(),
                currency:     "SEK".to_string(),
                bank_name:    "Nordea".to_string(),
                iban:         "SE3550000000054910000003".to_string(),
                transactions: Vec::new(),
            },
            SimulatedAccount {
                id:           "acc-revolut-001".to_string(),
                owner:        "LandveX AB".to_string(),
                balance:      Decimal::from_str("12480.75").unwrap(),
                currency:     "EUR".to_string(),
                bank_name:    "Revolut Business".to_string(),
                iban:         "GB29REVO00996912345678".to_string(),
                transactions: Vec::new(),
            },
            SimulatedAccount {
                id:           "acc-seb-001".to_string(),
                owner:        "LandveX AB".to_string(),
                balance:      Decimal::from_str("95000.00").unwrap(),
                currency:     "SEK".to_string(),
                bank_name:    "SEB".to_string(),
                iban:         "SE4550000000058398257466".to_string(),
                transactions: Vec::new(),
            },
        ];
        for acc in accounts {
            self.accounts.insert(acc.id.clone(), acc);
        }
    }

    // ── Transaction generation ───────────────────────────────────────────────

    /// Create a single transaction for a given account and merchant name.
    pub fn generate_transaction(
        &mut self,
        account_id: &str,
        merchant_name: &str,
        amount: Decimal,
    ) -> SimulatedTransaction {
        let template = MERCHANT_CATALOG
            .iter()
            .find(|m| m.name == merchant_name)
            .unwrap_or(&MERCHANT_CATALOG[0]);

        let txn = SimulatedTransaction::new(
            account_id,
            template,
            amount,
            Utc::now(),
            rand::thread_rng().gen_bool(0.75),
        );

        if let Some(acc) = self.accounts.get_mut(account_id) {
            acc.balance -= amount;
            acc.transactions.push(txn.clone());
        }
        txn
    }

    /// Generate `count` random daily transactions across all accounts.
    pub fn generate_daily_transactions(&mut self, count: usize) -> Vec<SimulatedTransaction> {
        let mut rng        = rand::thread_rng();
        let account_ids: Vec<String> = self.accounts.keys().cloned().collect();
        let mut results    = Vec::with_capacity(count);

        for _ in 0..count {
            let template   = &MERCHANT_CATALOG[rng.gen_range(0..MERCHANT_CATALOG.len())];
            let amount_f64 = rng.gen_range(template.min_amount..=template.max_amount);
            let amount     = Decimal::from_f64(amount_f64)
                .unwrap_or_default()
                .round_dp(2);
            let account_id = account_ids[rng.gen_range(0..account_ids.len())].clone();
            let minutes_back: i64 = rng.gen_range(0..1440);
            let timestamp  = Utc::now() - Duration::minutes(minutes_back);
            let receipt    = rng.gen_bool(0.72);

            let mut txn    = SimulatedTransaction::new(&account_id, template, amount, timestamp, receipt);
            if !rng.gen_bool(0.92) {
                txn.status = TxStatus::Pending;
            }

            if let Some(acc) = self.accounts.get_mut(&account_id) {
                acc.balance -= amount;
                acc.transactions.push(txn.clone());
            }
            results.push(txn);
        }
        results
    }

    // ── Webhook delivery ─────────────────────────────────────────────────────

    /// POST the transaction event to the configured webhook URL.
    /// Returns `Err` if delivery fails or webhook failures are being simulated.
    pub async fn fire_webhook(
        &mut self,
        txn: &SimulatedTransaction,
    ) -> Result<(), String> {
        if self.simulate_webhook_failures {
            self.failed_webhooks.push(txn.clone());
            return Err("Simulated webhook failure: connection timeout after 30s".to_string());
        }

        let url = match &self.webhook_url {
            Some(u) => u.clone(),
            None    => return Ok(()), // webhook not configured – not an error
        };

        self.request_count += 1;
        let retry = 0u32;

        let payload = WebhookPayload {
            event:       "transaction.created".to_string(),
            transaction: txn.clone(),
            account_id:  txn.account_id.clone(),
            timestamp:   Utc::now(),
            retry,
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| e.to_string())?;

        let resp = client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Webhook delivery failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Webhook rejected with HTTP {}", resp.status()));
        }
        Ok(())
    }

    // ── Chaos helpers ────────────────────────────────────────────────────────

    /// Inject a duplicate transaction (same reference, 3 minutes later).
    pub fn simulate_duplicate(
        &mut self,
    ) -> Option<(SimulatedTransaction, SimulatedTransaction)> {
        let account_ids: Vec<String> = self.accounts.keys().cloned().collect();
        if account_ids.is_empty() { return None; }

        let account_id = account_ids[0].clone();
        let template   = &MERCHANT_CATALOG[0]; // ICA Maxi
        let amount     = Decimal::from_str("523.50").unwrap();
        let reference  = "REF-DUPTEST1".to_string();

        let original = SimulatedTransaction {
            id:             Uuid::new_v4(),
            amount,
            currency:       template.currency.to_string(),
            merchant:       template.name.to_string(),
            category:       template.category.to_string(),
            vat_rate:       Decimal::from_f64(template.vat_rate).unwrap_or_default(),
            vat_amount:     (amount / Decimal::from_str("1.12").unwrap() * Decimal::from_str("0.12").unwrap()).round_dp(2),
            net_amount:     (amount / Decimal::from_str("1.12").unwrap()).round_dp(2),
            timestamp:      Utc::now() - Duration::minutes(5),
            status:         TxStatus::Completed,
            receipt_exists: true,
            account_id:     account_id.clone(),
            reference:      reference.clone(),
            description:    format!("Purchase at {}", template.name),
            is_duplicate:   false,
            webhook_fired:  true,
        };

        let duplicate = SimulatedTransaction {
            id:           Uuid::new_v4(),
            timestamp:    Utc::now() - Duration::minutes(2),
            reference:    reference.clone(), // same reference — key signal
            is_duplicate: true,
            ..original.clone()
        };

        if let Some(acc) = self.accounts.get_mut(&account_id) {
            acc.balance -= amount * Decimal::from(2);
            acc.transactions.push(original.clone());
            acc.transactions.push(duplicate.clone());
        }

        let pair = (original, duplicate);
        self.duplicate_pairs.push(pair.clone());
        Some(pair)
    }

    /// Start injecting webhook failures on every subsequent `fire_webhook` call.
    pub fn simulate_failed_webhook(&mut self) {
        self.simulate_webhook_failures = true;
    }

    /// Restore normal webhook delivery.
    pub fn restore_webhook(&mut self) {
        self.simulate_webhook_failures = false;
    }

    // ── Reporting ────────────────────────────────────────────────────────────

    pub fn get_account_summary(&self) -> Vec<AccountSummary> {
        self.accounts
            .values()
            .map(|acc| {
                let total_debited = acc
                    .transactions
                    .iter()
                    .filter(|t| t.status == TxStatus::Completed)
                    .map(|t| t.amount)
                    .fold(Decimal::ZERO, |s, a| s + a);

                let total_vat = acc
                    .transactions
                    .iter()
                    .filter(|t| t.status == TxStatus::Completed)
                    .map(|t| t.vat_amount)
                    .fold(Decimal::ZERO, |s, a| s + a);

                let pending_count = acc
                    .transactions
                    .iter()
                    .filter(|t| t.status == TxStatus::Pending)
                    .count();

                let missing_receipts = acc
                    .transactions
                    .iter()
                    .filter(|t| !t.receipt_exists && t.status == TxStatus::Completed)
                    .count();

                AccountSummary {
                    id:                acc.id.clone(),
                    owner:             acc.owner.clone(),
                    bank_name:         acc.bank_name.clone(),
                    balance:           acc.balance,
                    currency:          acc.currency.clone(),
                    transaction_count: acc.transactions.len(),
                    total_debited,
                    total_vat,
                    pending_count,
                    missing_receipts,
                }
            })
            .collect()
    }

    pub fn get_all_transactions(&self) -> Vec<SimulatedTransaction> {
        let mut txns: Vec<SimulatedTransaction> = self
            .accounts
            .values()
            .flat_map(|acc| acc.transactions.iter().cloned())
            .collect();
        txns.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        txns
    }

    pub fn find_transaction(&self, id: Uuid) -> Option<&SimulatedTransaction> {
        self.accounts
            .values()
            .flat_map(|acc| acc.transactions.iter())
            .find(|t| t.id == id)
    }

    /// Mark a completed transaction as refunded and restore the balance.
    pub fn mark_refunded(&mut self, id: Uuid) -> bool {
        for acc in self.accounts.values_mut() {
            if let Some(txn) = acc.transactions.iter_mut().find(|t| t.id == id) {
                if txn.status == TxStatus::Completed {
                    acc.balance += txn.amount;
                    txn.status   = TxStatus::Refunded;
                    return true;
                }
            }
        }
        false
    }

    /// Count transactions with a given status across all accounts.
    pub fn count_by_status(&self, status: TxStatus) -> usize {
        self.accounts
            .values()
            .flat_map(|acc| acc.transactions.iter())
            .filter(|t| t.status == status)
            .count()
    }

    /// Total balance across all accounts in their native currencies (not converted).
    pub fn all_balances(&self) -> Vec<(String, Decimal, String)> {
        self.accounts
            .values()
            .map(|acc| (acc.bank_name.clone(), acc.balance, acc.currency.clone()))
            .collect()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_three_accounts_created() {
        let sim = BankSimulator::new();
        assert_eq!(sim.accounts.len(), 3);
        assert!(sim.accounts.contains_key("acc-nordea-001"));
        assert!(sim.accounts.contains_key("acc-revolut-001"));
        assert!(sim.accounts.contains_key("acc-seb-001"));
    }

    #[test]
    fn test_generate_transaction_reduces_balance() {
        let mut sim        = BankSimulator::new();
        let initial_balance = sim.accounts["acc-nordea-001"].balance;
        let amount          = Decimal::from_str("250.00").unwrap();
        sim.generate_transaction("acc-nordea-001", "ICA Maxi Sundbyberg", amount);
        let new_balance     = sim.accounts["acc-nordea-001"].balance;
        assert_eq!(initial_balance - amount, new_balance);
    }

    #[test]
    fn test_duplicate_injection() {
        let mut sim = BankSimulator::new();
        let pair    = sim.simulate_duplicate().expect("duplicate pair should be created");
        assert_eq!(pair.0.reference, pair.1.reference);
        assert!(!pair.0.is_duplicate);
        assert!(pair.1.is_duplicate);
    }

    #[test]
    fn test_generate_daily_batch() {
        let mut sim  = BankSimulator::new();
        let txns     = sim.generate_daily_transactions(50);
        assert_eq!(txns.len(), 50);
        let all      = sim.get_all_transactions();
        assert_eq!(all.len(), 50);
    }

    #[test]
    fn test_mark_refunded() {
        let mut sim = BankSimulator::new();
        let txn     = sim.generate_transaction("acc-nordea-001", "ICA Maxi Sundbyberg", Decimal::from_str("200.00").unwrap());
        let balance_before_refund = sim.accounts["acc-nordea-001"].balance;
        assert!(sim.mark_refunded(txn.id));
        assert_eq!(
            sim.accounts["acc-nordea-001"].balance,
            balance_before_refund + Decimal::from_str("200.00").unwrap()
        );
    }

    #[test]
    fn test_merchant_catalog_coverage() {
        assert!(MERCHANT_CATALOG.len() >= 20, "Need at least 20 merchants");
        for m in MERCHANT_CATALOG {
            assert!(m.min_amount < m.max_amount, "merchant {} has invalid amount range", m.name);
            assert!(m.vat_rate > 0.0, "merchant {} has zero VAT rate", m.name);
        }
    }
}
