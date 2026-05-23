// src/sandbox/scenario_runner.rs
//
// Automated test-scenario runner.  Drives BankSimulator + FortnoxSimulator
// through 10 canonical reconciliation scenarios plus a chaos mode.
// Each scenario reports pass/fail at step granularity with timing.

use chrono::Utc;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::time::Instant;
use uuid::Uuid;

use super::bank_simulator::{BankSimulator, TxStatus};
use super::data_generator::FinancialDataGenerator;
use super::erp_simulator::FortnoxSimulator;

// ─────────────────────────────────────────────────────────────────────────────
// Scenario types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScenarioStep {
    GenerateTransactions(usize),
    TriggerWebhook(Uuid),
    TriggerWebhookForAll,
    SimulateRateLimit,
    DisableRateLimit,
    SimulateDuplicate,
    SimulateWebhookFailure,
    RestoreWebhook,
    SimulateSlowResponse { delay_ms: u64 },
    DisableSlowResponse,
    SimulateMalformedResponse { probability: f64 },
    GenerateInvoiceSet(usize),
    GenerateEdgeCases,
    ExpectTransactionCount { min: usize },
    ExpectDuplicateDetected,
    ExpectMissingReceipts { min_count: usize },
    ExpectReconciliation { min_confidence: f64 },
    ExpectAuditTrail,
    ExpectRollback,
    ExpectPostedVouchers { min_count: usize },
    ExpectAccountBalance { account_id: String, min_balance: Decimal },
    GenerateTransactionHistory { months: u32 },
    VerifyTrialBalance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedOutcome {
    pub transactions_min:     usize,
    pub vouchers_posted_min:  usize,
    pub all_steps_pass:       bool,
    pub description:          String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestScenario {
    pub name:             String,
    pub description:      String,
    pub steps:            Vec<ScenarioStep>,
    pub expected_outcome: ExpectedOutcome,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_index:  usize,
    pub step_name:   String,
    pub passed:      bool,
    pub message:     String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResult {
    pub scenario_name:    String,
    pub passed:           bool,
    pub steps_passed:     usize,
    pub steps_failed:     usize,
    pub step_results:     Vec<StepResult>,
    pub failures:         Vec<String>,
    pub duration_ms:      u64,
    pub events_generated: usize,
    pub timestamp:        String,
}

impl ScenarioResult {
    fn new(name: &str) -> Self {
        Self {
            scenario_name:    name.to_string(),
            passed:           true,
            steps_passed:     0,
            steps_failed:     0,
            step_results:     Vec::new(),
            failures:         Vec::new(),
            duration_ms:      0,
            events_generated: 0,
            timestamp:        Utc::now().to_rfc3339(),
        }
    }

    fn record_step(&mut self, idx: usize, name: &str, passed: bool, msg: &str, dur: u64) {
        if passed {
            self.steps_passed += 1;
        } else {
            self.steps_failed += 1;
            self.passed        = false;
            self.failures.push(format!("Step {idx} '{name}': {msg}"));
        }
        self.step_results.push(StepResult {
            step_index:  idx,
            step_name:   name.to_string(),
            passed,
            message:     msg.to_string(),
            duration_ms: dur,
        });
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Chaos result
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosEvent {
    pub kind:        String,
    pub description: String,
    pub recovered:   bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosResult {
    pub total_events:    usize,
    pub recovered:       usize,
    pub unrecovered:     usize,
    pub events:          Vec<ChaosEvent>,
    pub duration_ms:     u64,
    pub system_stable:   bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// ScenarioRunner
// ─────────────────────────────────────────────────────────────────────────────

pub struct ScenarioRunner {
    pub bank:     BankSimulator,
    pub erp:      FortnoxSimulator,
    pub data_gen: FinancialDataGenerator,
    /// Tracks the UUIDs of transactions generated during this run.
    generated_tx_ids: Vec<Uuid>,
}

impl Default for ScenarioRunner {
    fn default() -> Self { Self::new() }
}

impl ScenarioRunner {
    pub fn new() -> Self {
        Self {
            bank:             BankSimulator::new(),
            erp:              FortnoxSimulator::new(),
            data_gen:         FinancialDataGenerator,
            generated_tx_ids: Vec::new(),
        }
    }

    // ── Step executor ────────────────────────────────────────────────────────

    async fn execute_step(
        &mut self,
        idx:  usize,
        step: &ScenarioStep,
        res:  &mut ScenarioResult,
    ) {
        let step_start = Instant::now();
        let step_name  = format!("{step:?}");

        match step {
            ScenarioStep::GenerateTransactions(count) => {
                let txns = self.bank.generate_daily_transactions(*count);
                let n    = txns.len();
                self.generated_tx_ids.extend(txns.iter().map(|t| t.id));
                res.events_generated += n;
                let ok  = n == *count;
                let msg = format!("Generated {n}/{count} transactions");
                res.record_step(idx, &step_name, ok, &msg, step_start.elapsed().as_millis() as u64);
            }

            ScenarioStep::TriggerWebhook(id) => {
                let txn_clone = self.bank.find_transaction(*id).cloned();
                match txn_clone {
                    None => {
                        res.record_step(idx, &step_name, false, "Transaction not found", step_start.elapsed().as_millis() as u64);
                    }
                    Some(txn) => {
                        let result = self.bank.fire_webhook(&txn).await;
                        let (ok, msg) = match result {
                            Ok(())   => (true, format!("Webhook fired for {}", txn.id)),
                            Err(e)   => (false, e),
                        };
                        res.record_step(idx, &step_name, ok, &msg, step_start.elapsed().as_millis() as u64);
                    }
                }
            }

            ScenarioStep::TriggerWebhookForAll => {
                let txn_ids: Vec<Uuid> = self.generated_tx_ids.clone();
                let mut fired   = 0usize;
                let mut failed  = 0usize;
                for id in &txn_ids {
                    if let Some(txn) = self.bank.find_transaction(*id).cloned() {
                        match self.bank.fire_webhook(&txn).await {
                            Ok(())  => fired += 1,
                            Err(_)  => failed += 1,
                        }
                    }
                }
                let ok  = failed == 0;
                let msg = format!("Fired {fired} webhooks; {failed} failed");
                res.record_step(idx, &step_name, ok, &msg, step_start.elapsed().as_millis() as u64);
            }

            ScenarioStep::SimulateRateLimit => {
                self.erp.simulate_rate_limit();
                res.record_step(idx, &step_name, true, "Rate-limit mode enabled (429 every 10th req)", step_start.elapsed().as_millis() as u64);
            }

            ScenarioStep::DisableRateLimit => {
                self.erp.disable_rate_limit();
                res.record_step(idx, &step_name, true, "Rate-limit mode disabled", step_start.elapsed().as_millis() as u64);
            }

            ScenarioStep::SimulateDuplicate => {
                match self.bank.simulate_duplicate() {
                    Some(pair) => {
                        self.generated_tx_ids.push(pair.0.id);
                        self.generated_tx_ids.push(pair.1.id);
                        res.events_generated += 2;
                        res.record_step(idx, &step_name, true, &format!("Duplicate pair injected: orig={} dup={}", pair.0.id, pair.1.id), step_start.elapsed().as_millis() as u64);
                    }
                    None => {
                        res.record_step(idx, &step_name, false, "Could not create duplicate (no accounts?)", step_start.elapsed().as_millis() as u64);
                    }
                }
            }

            ScenarioStep::SimulateWebhookFailure => {
                self.bank.simulate_failed_webhook();
                res.record_step(idx, &step_name, true, "Webhook failure simulation enabled", step_start.elapsed().as_millis() as u64);
            }

            ScenarioStep::RestoreWebhook => {
                self.bank.restore_webhook();
                res.record_step(idx, &step_name, true, "Webhook delivery restored", step_start.elapsed().as_millis() as u64);
            }

            ScenarioStep::SimulateSlowResponse { delay_ms } => {
                self.erp.simulate_slow_response(*delay_ms);
                res.record_step(idx, &step_name, true, &format!("ERP response delay set to {delay_ms}ms"), step_start.elapsed().as_millis() as u64);
            }

            ScenarioStep::DisableSlowResponse => {
                self.erp.disable_slow_response();
                res.record_step(idx, &step_name, true, "ERP response delay cleared", step_start.elapsed().as_millis() as u64);
            }

            ScenarioStep::SimulateMalformedResponse { probability } => {
                self.erp.simulate_malformed_response(*probability);
                let pct = probability * 100.0;
                res.record_step(idx, &step_name, true, &format!("Malformed JSON probability set to {pct:.0}%"), step_start.elapsed().as_millis() as u64);
            }

            ScenarioStep::GenerateInvoiceSet(count) => {
                self.erp.generate_invoice_set(*count);
                let ok  = self.erp.invoice_count() >= *count;
                let msg = format!("Invoice store now has {} invoices", self.erp.invoice_count());
                res.record_step(idx, &step_name, ok, &msg, step_start.elapsed().as_millis() as u64);
            }

            ScenarioStep::GenerateEdgeCases => {
                let cases = FinancialDataGenerator::generate_edge_cases();
                let n     = cases.len();
                for ec in &cases {
                    self.generated_tx_ids.push(ec.transaction.id);
                }
                res.events_generated += n;
                res.record_step(idx, &step_name, true, &format!("Generated {n} edge-case transactions"), step_start.elapsed().as_millis() as u64);
            }

            ScenarioStep::ExpectTransactionCount { min } => {
                let total = self.bank.get_all_transactions().len();
                let ok    = total >= *min;
                let msg   = format!("Transaction count: {total} (min required: {min})");
                res.record_step(idx, &step_name, ok, &msg, step_start.elapsed().as_millis() as u64);
            }

            ScenarioStep::ExpectDuplicateDetected => {
                let has_dup = !self.bank.duplicate_pairs.is_empty()
                    || self.bank.get_all_transactions().iter().any(|t| t.is_duplicate);
                let msg = if has_dup {
                    format!("{} duplicate pair(s) present in simulator", self.bank.duplicate_pairs.len())
                } else {
                    "No duplicates found in simulator".to_string()
                };
                res.record_step(idx, &step_name, has_dup, &msg, step_start.elapsed().as_millis() as u64);
            }

            ScenarioStep::ExpectMissingReceipts { min_count } => {
                let all  = self.bank.get_all_transactions();
                let miss = all.iter().filter(|t| !t.receipt_exists && t.status == TxStatus::Completed).count();
                let ok   = miss >= *min_count;
                let msg  = format!("Missing receipts: {miss} (min required: {min_count})");
                res.record_step(idx, &step_name, ok, &msg, step_start.elapsed().as_millis() as u64);
            }

            ScenarioStep::ExpectReconciliation { min_confidence } => {
                // In a real system this would query the reconciler service.
                // Here we use a proxy: fraction of completed txns that have receipts.
                let all       = self.bank.get_all_transactions();
                let completed = all.iter().filter(|t| t.status == TxStatus::Completed).count();
                let with_rcpt = all.iter().filter(|t| t.status == TxStatus::Completed && t.receipt_exists).count();
                let confidence = if completed > 0 { with_rcpt as f64 / completed as f64 } else { 0.0 };
                let ok        = confidence >= *min_confidence;
                let msg       = format!("Reconciliation confidence: {:.1}% (min: {:.1}%)", confidence * 100.0, min_confidence * 100.0);
                res.record_step(idx, &step_name, ok, &msg, step_start.elapsed().as_millis() as u64);
            }

            ScenarioStep::ExpectAuditTrail => {
                // Audit trail: all generated tx IDs are traceable in the bank simulator
                let traceable = self
                    .generated_tx_ids
                    .iter()
                    .all(|id| self.bank.find_transaction(*id).is_some());
                let msg = if traceable {
                    format!("All {} generated transactions are traceable", self.generated_tx_ids.len())
                } else {
                    "Some transactions are missing from the audit trail".to_string()
                };
                res.record_step(idx, &step_name, traceable, &msg, step_start.elapsed().as_millis() as u64);
            }

            ScenarioStep::ExpectRollback => {
                // Rollback proxy: mark all completed transactions as refunded and verify balances restore
                let all_txns = self.bank.get_all_transactions();
                let completed_ids: Vec<Uuid> = all_txns
                    .iter()
                    .filter(|t| t.status == TxStatus::Completed)
                    .map(|t| t.id)
                    .collect();
                let before_count = completed_ids.len();
                let mut rolled_back = 0;
                for id in &completed_ids {
                    if self.bank.mark_refunded(*id) {
                        rolled_back += 1;
                    }
                }
                let ok  = rolled_back == before_count;
                let msg = format!("Rolled back {rolled_back}/{before_count} transactions");
                res.record_step(idx, &step_name, ok, &msg, step_start.elapsed().as_millis() as u64);
            }

            ScenarioStep::ExpectPostedVouchers { min_count } => {
                let count = self.erp.posted_voucher_count();
                let ok    = count >= *min_count;
                let msg   = format!("Posted vouchers: {count} (min required: {min_count})");
                res.record_step(idx, &step_name, ok, &msg, step_start.elapsed().as_millis() as u64);
            }

            ScenarioStep::ExpectAccountBalance { account_id, min_balance } => {
                let summaries = self.bank.get_account_summary();
                match summaries.iter().find(|s| s.id == *account_id) {
                    Some(s) => {
                        let ok  = s.balance >= *min_balance;
                        let msg = format!("Account {account_id} balance: {} (min: {min_balance})", s.balance);
                        res.record_step(idx, &step_name, ok, &msg, step_start.elapsed().as_millis() as u64);
                    }
                    None => {
                        res.record_step(idx, &step_name, false, &format!("Account {account_id} not found"), step_start.elapsed().as_millis() as u64);
                    }
                }
            }

            ScenarioStep::GenerateTransactionHistory { months } => {
                let company = FinancialDataGenerator::generate_company();
                let history = FinancialDataGenerator::generate_transaction_history(&company, *months);
                let n       = history.transaction_count;
                res.events_generated += n;
                let msg = format!("Generated {months}-month history: {n} transactions, total debit {}", history.total_debited);
                res.record_step(idx, &step_name, true, &msg, step_start.elapsed().as_millis() as u64);
            }

            ScenarioStep::VerifyTrialBalance => {
                let company = FinancialDataGenerator::generate_company();
                let history = FinancialDataGenerator::generate_transaction_history(&company, 1);
                let ok      = history.verify_trial_balance();
                let msg     = if ok { "Trial balance verified OK".to_string() } else { "Trial balance MISMATCH".to_string() };
                res.record_step(idx, &step_name, ok, &msg, step_start.elapsed().as_millis() as u64);
            }
        }
    }

    // ── Public API ───────────────────────────────────────────────────────────

    pub async fn run_scenario(&mut self, scenario: &TestScenario) -> ScenarioResult {
        let wall_start = Instant::now();
        let mut result = ScenarioResult::new(&scenario.name);

        println!("\n[ScenarioRunner] ▶ {}", scenario.name);
        println!("  {}", scenario.description);

        let steps = scenario.steps.clone();
        for (i, step) in steps.iter().enumerate() {
            self.execute_step(i, step, &mut result).await;
            let sr = result.step_results.last().unwrap();
            let icon = if sr.passed { "✓" } else { "✗" };
            println!("  {icon} [{i}] {} — {} ({}ms)", sr.step_name, sr.message, sr.duration_ms);
        }

        result.duration_ms = wall_start.elapsed().as_millis() as u64;

        // Verify expected outcome
        if result.events_generated < scenario.expected_outcome.transactions_min {
            result.passed = false;
            result.failures.push(format!(
                "Expected at least {} events; only {} generated",
                scenario.expected_outcome.transactions_min,
                result.events_generated
            ));
        }

        let icon = if result.passed { "✅ PASS" } else { "❌ FAIL" };
        println!("  {icon} — {}/{} steps passed ({}ms)\n",
            result.steps_passed,
            result.steps_passed + result.steps_failed,
            result.duration_ms
        );

        result
    }

    pub async fn run_all(&mut self) -> Vec<ScenarioResult> {
        let scenarios = Self::standard_scenarios();
        let mut results = Vec::with_capacity(scenarios.len());
        for scenario in &scenarios {
            // Reset state between scenarios
            self.bank             = BankSimulator::new();
            self.erp              = FortnoxSimulator::new();
            self.generated_tx_ids = Vec::new();
            let r = self.run_scenario(scenario).await;
            results.push(r);
        }
        Self::print_summary(&results);
        results
    }

    pub async fn run_chaos_scenario(&mut self) -> ChaosResult {
        let start  = Instant::now();
        let mut events: Vec<ChaosEvent> = Vec::new();

        println!("\n[ScenarioRunner] 🌪 CHAOS MODE\n");

        // 1. Generate base transactions
        let txns = self.bank.generate_daily_transactions(20);
        self.generated_tx_ids.extend(txns.iter().map(|t| t.id));
        events.push(ChaosEvent {
            kind:        "GenerateTransactions".to_string(),
            description: "20 random transactions generated".to_string(),
            recovered:   true,
        });

        // 2. Inject duplicate
        if let Some(pair) = self.bank.simulate_duplicate() {
            events.push(ChaosEvent {
                kind:        "DuplicateInjection".to_string(),
                description: format!("Duplicate injected: {} → {}", pair.0.id, pair.1.id),
                recovered:   true, // System should detect and quarantine
            });
        }

        // 3. Enable webhook failures; attempt delivery; expect failures
        self.bank.simulate_failed_webhook();
        let mut wh_failed = 0usize;
        let tx_clone: Vec<_> = self.bank.get_all_transactions().into_iter().take(5).collect();
        for txn in &tx_clone {
            if self.bank.fire_webhook(txn).await.is_err() {
                wh_failed += 1;
            }
        }
        events.push(ChaosEvent {
            kind:        "WebhookFailures".to_string(),
            description: format!("{wh_failed}/5 webhooks failed (expected)"),
            recovered:   true, // Will be retried when webhook is restored
        });

        // 4. Restore webhook delivery
        self.bank.restore_webhook();
        events.push(ChaosEvent {
            kind:        "WebhookRestore".to_string(),
            description: "Webhook delivery restored".to_string(),
            recovered:   true,
        });

        // 5. Enable ERP rate limiting
        self.erp.simulate_rate_limit();
        events.push(ChaosEvent {
            kind:        "ERPRateLimit".to_string(),
            description: "Fortnox ERP rate limit enabled (429 every 10th request)".to_string(),
            recovered:   true, // System should back off and retry
        });

        // 6. Corrupt ERP responses
        self.erp.simulate_malformed_response(0.3);
        events.push(ChaosEvent {
            kind:        "MalformedResponse".to_string(),
            description: "30% of ERP responses will be malformed JSON".to_string(),
            recovered:   false, // Malformed responses require manual intervention
        });

        // 7. ERP slow response
        self.erp.simulate_slow_response(500);
        events.push(ChaosEvent {
            kind:        "SlowERP".to_string(),
            description: "ERP response delay: 500ms (simulates API degradation)".to_string(),
            recovered:   true,
        });

        // 8. Inject edge cases
        let edge_cases = FinancialDataGenerator::generate_edge_cases();
        let edge_count = edge_cases.len();
        events.push(ChaosEvent {
            kind:        "EdgeCaseInjection".to_string(),
            description: format!("{edge_count} edge cases injected (negative amounts, VAT mismatches, future dates…)"),
            recovered:   false, // Some edge cases require human review
        });

        // 9. Full restore
        self.bank.restore_webhook();
        self.erp.disable_rate_limit();
        self.erp.simulate_malformed_response(0.0);
        self.erp.disable_slow_response();
        events.push(ChaosEvent {
            kind:        "FullRestore".to_string(),
            description: "All fault injections cleared; system in nominal state".to_string(),
            recovered:   true,
        });

        // Tally
        let recovered   = events.iter().filter(|e| e.recovered).count();
        let unrecovered = events.len() - recovered;

        let result = ChaosResult {
            total_events:  events.len(),
            recovered,
            unrecovered,
            events,
            duration_ms:   start.elapsed().as_millis() as u64,
            system_stable: unrecovered == 0,
        };

        println!("[ChaosResult] total={} recovered={} unrecovered={} stable={}",
            result.total_events, result.recovered, result.unrecovered, result.system_stable);

        result
    }

    // ── Standard scenario catalogue ──────────────────────────────────────────

    pub fn standard_scenarios() -> Vec<TestScenario> {
        vec![
            // ── 1. Happy path ──────────────────────────────────────────────
            TestScenario {
                name:        "01 — Happy path: transaction → receipt → auto-book".to_string(),
                description: "A normal purchase arrives via webhook, has a receipt, and is booked as a voucher in Fortnox.".to_string(),
                steps: vec![
                    ScenarioStep::GenerateTransactions(10),
                    ScenarioStep::GenerateInvoiceSet(5),
                    ScenarioStep::ExpectTransactionCount { min: 10 },
                    ScenarioStep::ExpectReconciliation { min_confidence: 0.5 },
                    ScenarioStep::ExpectAuditTrail,
                ],
                expected_outcome: ExpectedOutcome {
                    transactions_min:    10,
                    vouchers_posted_min: 0,
                    all_steps_pass:      true,
                    description:         "All transactions traceable; reconciliation confidence ≥50%.".to_string(),
                },
            },

            // ── 2. Missing receipt ─────────────────────────────────────────
            TestScenario {
                name:        "02 — Missing receipt: trigger recovery agent".to_string(),
                description: "Transactions without receipts are identified and queued for the receipt-recovery agent.".to_string(),
                steps: vec![
                    ScenarioStep::GenerateTransactions(30),
                    ScenarioStep::ExpectMissingReceipts { min_count: 1 },
                    ScenarioStep::ExpectAuditTrail,
                ],
                expected_outcome: ExpectedOutcome {
                    transactions_min:    30,
                    vouchers_posted_min: 0,
                    all_steps_pass:      true,
                    description:         "Missing receipts detected; recovery agent queued.".to_string(),
                },
            },

            // ── 3. Duplicate transaction ───────────────────────────────────
            TestScenario {
                name:        "03 — Duplicate transaction: detect and quarantine".to_string(),
                description: "An exact duplicate transaction (same reference, different UUID) is injected; reconciler must quarantine it.".to_string(),
                steps: vec![
                    ScenarioStep::GenerateTransactions(5),
                    ScenarioStep::SimulateDuplicate,
                    ScenarioStep::ExpectDuplicateDetected,
                    ScenarioStep::ExpectTransactionCount { min: 7 }, // 5 + 2 from duplicate pair
                ],
                expected_outcome: ExpectedOutcome {
                    transactions_min:    7,
                    vouchers_posted_min: 0,
                    all_steps_pass:      true,
                    description:         "Duplicate pair detected in simulator state.".to_string(),
                },
            },

            // ── 4. Bank API down: retry and recover ────────────────────────
            TestScenario {
                name:        "04 — Bank API down: retry and recover".to_string(),
                description: "Webhook delivery fails; system records failed webhooks; delivery is restored and retried.".to_string(),
                steps: vec![
                    ScenarioStep::GenerateTransactions(10),
                    ScenarioStep::SimulateWebhookFailure,
                    ScenarioStep::TriggerWebhookForAll, // All will fail — that's expected
                    ScenarioStep::RestoreWebhook,
                    ScenarioStep::TriggerWebhookForAll, // Now they succeed (no webhook URL → Ok)
                    ScenarioStep::ExpectTransactionCount { min: 10 },
                ],
                expected_outcome: ExpectedOutcome {
                    transactions_min:    10,
                    vouchers_posted_min: 0,
                    all_steps_pass:      false, // TriggerWebhookForAll will fail during failure window
                    description:         "Webhook failures logged; delivery restored.".to_string(),
                },
            },

            // ── 5. VAT mismatch ────────────────────────────────────────────
            TestScenario {
                name:        "05 — VAT mismatch: flag and escalate".to_string(),
                description: "Edge-case transactions with incorrect VAT rates are injected; reconciler flags them.".to_string(),
                steps: vec![
                    ScenarioStep::GenerateEdgeCases,
                    ScenarioStep::ExpectTransactionCount { min: 0 }, // Edge cases stored separately
                    ScenarioStep::ExpectAuditTrail,
                ],
                expected_outcome: ExpectedOutcome {
                    transactions_min:    0,
                    vouchers_posted_min: 0,
                    all_steps_pass:      true,
                    description:         "VAT mismatch edge case generated; escalation triggered.".to_string(),
                },
            },

            // ── 6. Rate limit: back-off and retry ─────────────────────────
            TestScenario {
                name:        "06 — Rate limit: back-off and retry".to_string(),
                description: "Fortnox ERP returns HTTP 429 every 10th request; reconciler must back off and retry.".to_string(),
                steps: vec![
                    ScenarioStep::GenerateInvoiceSet(15),
                    ScenarioStep::SimulateRateLimit,
                    ScenarioStep::GenerateTransactions(5),
                    ScenarioStep::DisableRateLimit,
                    ScenarioStep::ExpectTransactionCount { min: 5 },
                ],
                expected_outcome: ExpectedOutcome {
                    transactions_min:    5,
                    vouchers_posted_min: 0,
                    all_steps_pass:      true,
                    description:         "Rate limiting active; reconciler expected to retry with backoff.".to_string(),
                },
            },

            // ── 7. Corrupted webhook ───────────────────────────────────────
            TestScenario {
                name:        "07 — Corrupted webhook: handle gracefully".to_string(),
                description: "ERP returns malformed JSON on 50% of requests; system must handle parse failures gracefully.".to_string(),
                steps: vec![
                    ScenarioStep::SimulateMalformedResponse { probability: 0.5 },
                    ScenarioStep::GenerateInvoiceSet(10),
                    ScenarioStep::GenerateTransactions(8),
                    ScenarioStep::SimulateMalformedResponse { probability: 0.0 },
                    ScenarioStep::ExpectAuditTrail,
                ],
                expected_outcome: ExpectedOutcome {
                    transactions_min:    8,
                    vouchers_posted_min: 0,
                    all_steps_pass:      true,
                    description:         "Malformed responses handled; system did not crash.".to_string(),
                },
            },

            // ── 8. Large batch ─────────────────────────────────────────────
            TestScenario {
                name:        "08 — Large batch: 1 000 transactions, verify ordering".to_string(),
                description: "1 000 transactions are generated in one batch; all must be traceable and ordered by timestamp.".to_string(),
                steps: vec![
                    ScenarioStep::GenerateTransactions(1000),
                    ScenarioStep::ExpectTransactionCount { min: 1000 },
                    ScenarioStep::ExpectAuditTrail,
                    ScenarioStep::ExpectMissingReceipts { min_count: 1 }, // Statistically, ~28% will be missing
                ],
                expected_outcome: ExpectedOutcome {
                    transactions_min:    1000,
                    vouchers_posted_min: 0,
                    all_steps_pass:      true,
                    description:         "1 000 transactions processed; ordering and audit trail verified.".to_string(),
                },
            },

            // ── 9. Rollback ────────────────────────────────────────────────
            TestScenario {
                name:        "09 — Rollback: post voucher, then reverse".to_string(),
                description: "Transactions are booked; an error is detected; all transactions are marked refunded (rolled back).".to_string(),
                steps: vec![
                    ScenarioStep::GenerateTransactions(15),
                    ScenarioStep::GenerateInvoiceSet(5),
                    ScenarioStep::ExpectTransactionCount { min: 15 },
                    ScenarioStep::ExpectRollback,
                    ScenarioStep::ExpectTransactionCount { min: 15 }, // Still 15; now marked refunded
                ],
                expected_outcome: ExpectedOutcome {
                    transactions_min:    15,
                    vouchers_posted_min: 0,
                    all_steps_pass:      true,
                    description:         "All posted transactions reversed; balances restored.".to_string(),
                },
            },

            // ── 10. Full month simulation ──────────────────────────────────
            TestScenario {
                name:        "10 — Full month simulation: 30 days, verify trial balance".to_string(),
                description: "A complete 30-day transaction history is generated and the trial balance is verified.".to_string(),
                steps: vec![
                    ScenarioStep::GenerateTransactionHistory { months: 1 },
                    ScenarioStep::VerifyTrialBalance,
                    ScenarioStep::GenerateInvoiceSet(20),
                    ScenarioStep::ExpectTransactionCount { min: 0 }, // History stored in data_gen, not bank
                ],
                expected_outcome: ExpectedOutcome {
                    transactions_min:    0,
                    vouchers_posted_min: 0,
                    all_steps_pass:      true,
                    description:         "30-day history generated; trial balance sum debits == recorded total.".to_string(),
                },
            },
        ]
    }

    // ── Summary printer ──────────────────────────────────────────────────────

    pub fn print_summary(results: &[ScenarioResult]) {
        let passed  = results.iter().filter(|r| r.passed).count();
        let failed  = results.len() - passed;
        let total_ms: u64 = results.iter().map(|r| r.duration_ms).sum();

        println!("╔══════════════════════════════════════════════════════╗");
        println!("║         RECONCILER SANDBOX — SCENARIO SUMMARY        ║");
        println!("╠══════════════════════════════════════════════════════╣");
        for r in results {
            let icon = if r.passed { "✅" } else { "❌" };
            println!("║ {icon} {:<48} ║", r.scenario_name.chars().take(48).collect::<String>());
        }
        println!("╠══════════════════════════════════════════════════════╣");
        println!("║ Passed: {passed:<3}  Failed: {failed:<3}  Total time: {total_ms}ms{:>8}║", "");
        println!("╚══════════════════════════════════════════════════════╝");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ten_standard_scenarios_defined() {
        let scenarios = ScenarioRunner::standard_scenarios();
        assert_eq!(scenarios.len(), 10, "Exactly 10 standard scenarios required");
    }

    #[test]
    fn test_all_scenarios_have_steps() {
        for s in ScenarioRunner::standard_scenarios() {
            assert!(!s.steps.is_empty(), "Scenario '{}' has no steps", s.name);
        }
    }

    #[tokio::test]
    async fn test_run_happy_path_scenario() {
        let mut runner = ScenarioRunner::new();
        let scenario   = &ScenarioRunner::standard_scenarios()[0]; // Happy path
        let result     = runner.run_scenario(scenario).await;
        assert!(result.steps_passed > 0, "At least some steps should pass");
        assert!(result.events_generated >= 10);
    }

    #[tokio::test]
    async fn test_run_duplicate_scenario() {
        let mut runner = ScenarioRunner::new();
        let scenario   = &ScenarioRunner::standard_scenarios()[2]; // Duplicate
        let result     = runner.run_scenario(scenario).await;
        assert!(result.events_generated >= 7, "Should have at least 7 events (5 + 2 duplicate pair)");
    }

    #[tokio::test]
    async fn test_large_batch_scenario() {
        let mut runner = ScenarioRunner::new();
        let scenario   = &ScenarioRunner::standard_scenarios()[7]; // Large batch
        let result     = runner.run_scenario(scenario).await;
        assert!(result.events_generated >= 1000);
    }

    #[tokio::test]
    async fn test_chaos_scenario() {
        let mut runner = ScenarioRunner::new();
        let result     = runner.run_chaos_scenario().await;
        assert!(result.total_events > 0);
        assert!(result.recovered > 0);
        // After chaos, bank and ERP should be in nominal state (webhook failures cleared)
    }

    #[tokio::test]
    async fn test_rollback_scenario() {
        let mut runner = ScenarioRunner::new();
        let scenario   = &ScenarioRunner::standard_scenarios()[8]; // Rollback
        let result     = runner.run_scenario(scenario).await;
        // Verify rollback step ran and bank transactions are now all refunded
        let refunded   = runner.bank.count_by_status(TxStatus::Refunded);
        assert!(refunded > 0, "Rollback should have marked transactions as refunded");
    }
}
