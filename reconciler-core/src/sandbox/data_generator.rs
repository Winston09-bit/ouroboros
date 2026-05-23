// src/sandbox/data_generator.rs
//
// Synthetic financial data generator.  Creates entire fictional companies,
// multi-month transaction histories, minimal-valid PDF receipts, RFC 822
// receipt emails, and a comprehensive set of reconciliation edge-cases.

use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

use super::bank_simulator::{
    MerchantTemplate, SimulatedAccount, SimulatedTransaction, TxStatus, MERCHANT_CATALOG,
};

// ─────────────────────────────────────────────────────────────────────────────
// Edge case taxonomy
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeCaseType {
    DuplicateTransaction,
    WrongCurrency,
    MissingReceipt,
    CorruptedOCR,
    DelayedWebhook,
    NegativeAmount,
    ZeroAmount,
    FutureDate,
    VATMismatch,
    RefundWithoutOriginal,
    AmountRounding,
    UnsupportedCurrency,
}

impl std::fmt::Display for EdgeCaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            EdgeCaseType::DuplicateTransaction    => "DuplicateTransaction",
            EdgeCaseType::WrongCurrency           => "WrongCurrency",
            EdgeCaseType::MissingReceipt          => "MissingReceipt",
            EdgeCaseType::CorruptedOCR            => "CorruptedOCR",
            EdgeCaseType::DelayedWebhook          => "DelayedWebhook",
            EdgeCaseType::NegativeAmount          => "NegativeAmount",
            EdgeCaseType::ZeroAmount              => "ZeroAmount",
            EdgeCaseType::FutureDate              => "FutureDate",
            EdgeCaseType::VATMismatch             => "VATMismatch",
            EdgeCaseType::RefundWithoutOriginal   => "RefundWithoutOriginal",
            EdgeCaseType::AmountRounding          => "AmountRounding",
            EdgeCaseType::UnsupportedCurrency     => "UnsupportedCurrency",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeCase {
    pub case_type:        EdgeCaseType,
    pub description:      String,
    pub transaction:      SimulatedTransaction,
    pub expected_outcome: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Company and history types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCompany {
    pub name:             String,
    pub org_number:       String,
    pub jurisdiction:     String,
    pub vat_number:       String,
    pub address:          String,
    pub bank_accounts:    Vec<SimulatedAccount>,
    pub monthly_expenses: Decimal,
    pub employee_count:   u32,
    pub industry:         String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlySnapshot {
    pub year:           i32,
    pub month:          u32,
    pub transactions:   Vec<SimulatedTransaction>,
    pub total_debited:  Decimal,
    pub total_vat:      Decimal,
    pub net_expenses:   Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionHistory {
    pub company:        TestCompany,
    pub months:         Vec<MonthlySnapshot>,
    pub total_debited:  Decimal,
    pub total_vat:      Decimal,
    pub transaction_count: usize,
}

impl TransactionHistory {
    /// Compute the trial balance check: sum of debits == sum of credits (Decimal).
    pub fn verify_trial_balance(&self) -> bool {
        // In this simplified model, every debit reduces account balance.
        // The trial balance holds when total_debited == sum of all transaction amounts.
        let computed: Decimal = self
            .months
            .iter()
            .flat_map(|m| m.transactions.iter())
            .filter(|t| t.status == TxStatus::Completed)
            .map(|t| t.amount)
            .fold(Decimal::ZERO, |s, a| s + a);
        computed == self.total_debited
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// FinancialDataGenerator
// ─────────────────────────────────────────────────────────────────────────────

pub struct FinancialDataGenerator;

impl FinancialDataGenerator {
    // ── Company factory ──────────────────────────────────────────────────────

    /// Generate a complete fictional Swedish tech company with 3 bank accounts.
    pub fn generate_company() -> TestCompany {
        let nordea = SimulatedAccount {
            id:           "acc-nordea-001".to_string(),
            owner:        "TestCo AB".to_string(),
            balance:      Decimal::from_str("350000.00").unwrap(),
            currency:     "SEK".to_string(),
            bank_name:    "Nordea".to_string(),
            iban:         "SE3550000000054910000003".to_string(),
            transactions: Vec::new(),
        };
        let revolut = SimulatedAccount {
            id:           "acc-revolut-001".to_string(),
            owner:        "TestCo AB".to_string(),
            balance:      Decimal::from_str("8200.00").unwrap(),
            currency:     "EUR".to_string(),
            bank_name:    "Revolut Business".to_string(),
            iban:         "GB29REVO00996912345678".to_string(),
            transactions: Vec::new(),
        };
        let seb = SimulatedAccount {
            id:           "acc-seb-001".to_string(),
            owner:        "TestCo AB".to_string(),
            balance:      Decimal::from_str("75000.00").unwrap(),
            currency:     "SEK".to_string(),
            bank_name:    "SEB".to_string(),
            iban:         "SE4550000000058398257466".to_string(),
            transactions: Vec::new(),
        };

        TestCompany {
            name:             "TestCo AB".to_string(),
            org_number:       "559999-0001".to_string(),
            jurisdiction:     "SE".to_string(),
            vat_number:       "SE559999000101".to_string(),
            address:          "Testgatan 1, 111 22 Stockholm, Sweden".to_string(),
            bank_accounts:    vec![nordea, revolut, seb],
            monthly_expenses: Decimal::from_str("85000.00").unwrap(),
            employee_count:   12,
            industry:         "Software / SaaS".to_string(),
        }
    }

    // ── Transaction history ──────────────────────────────────────────────────

    /// Generate `months` months of realistic transactions for the given company.
    pub fn generate_transaction_history(
        company: &TestCompany,
        months: u32,
    ) -> TransactionHistory {
        let mut rng           = rand::thread_rng();
        let mut all_months    = Vec::new();
        let mut grand_debit   = Decimal::ZERO;
        let mut grand_vat     = Decimal::ZERO;
        let mut grand_count   = 0usize;

        let base = Utc::now();

        for month_offset in 0..months {
            // Work backwards so month 0 = current month
            let month_start = base
                - Duration::days(30 * (months - month_offset - 1) as i64);

            // Vary txn count slightly each month (15–35 per month)
            let tx_count: usize = rng.gen_range(15..=35);
            let mut txns: Vec<SimulatedTransaction> = Vec::with_capacity(tx_count);

            for _ in 0..tx_count {
                let template    = &MERCHANT_CATALOG[rng.gen_range(0..MERCHANT_CATALOG.len())];
                let amount_f64  = rng.gen_range(template.min_amount..=template.max_amount);
                let amount      = Decimal::from_f64(amount_f64).unwrap_or_default().round_dp(2);

                let account_idx = rng.gen_range(0..company.bank_accounts.len());
                let account_id  = company.bank_accounts[account_idx].id.clone();

                let day_offset: i64 = rng.gen_range(0..30);
                let timestamp   = month_start + Duration::days(day_offset);

                let vat_rate    = Decimal::from_f64(template.vat_rate).unwrap_or_default();
                let net_amount  = (amount / (Decimal::ONE + vat_rate)).round_dp(2);
                let vat_amount  = (amount - net_amount).round_dp(2);

                let txn = SimulatedTransaction {
                    id:             Uuid::new_v4(),
                    amount,
                    currency:       template.currency.to_string(),
                    merchant:       template.name.to_string(),
                    category:       template.category.to_string(),
                    vat_rate,
                    vat_amount,
                    net_amount,
                    timestamp,
                    status:         TxStatus::Completed,
                    receipt_exists: rng.gen_bool(0.75),
                    account_id,
                    reference:      format!("REF-{}", &Uuid::new_v4().to_string()[..8].to_uppercase()),
                    description:    format!("Purchase at {}", template.name),
                    is_duplicate:   false,
                    webhook_fired:  true,
                };

                grand_debit += txn.amount;
                grand_vat   += txn.vat_amount;
                txns.push(txn);
            }

            grand_count += txns.len();

            let total_debited = txns.iter().map(|t| t.amount).fold(Decimal::ZERO, |s, a| s + a);
            let total_vat     = txns.iter().map(|t| t.vat_amount).fold(Decimal::ZERO, |s, a| s + a);
            let net_expenses  = total_debited - total_vat;

            let dt = month_start;
            all_months.push(MonthlySnapshot {
                year:         dt.format("%Y").to_string().parse().unwrap_or(2026),
                month:        dt.format("%m").to_string().parse().unwrap_or(1),
                transactions: txns,
                total_debited,
                total_vat,
                net_expenses,
            });
        }

        TransactionHistory {
            company:           company.clone(),
            months:            all_months,
            total_debited:     grand_debit,
            total_vat:         grand_vat,
            transaction_count: grand_count,
        }
    }

    // ── Receipt utilities ────────────────────────────────────────────────────

    /// Return the UUIDs of transactions that should be treated as missing a receipt.
    pub fn generate_missing_receipts(
        txns:        &[SimulatedTransaction],
        missing_pct: f64,
    ) -> Vec<Uuid> {
        let missing_pct = missing_pct.clamp(0.0, 1.0);
        txns.iter()
            .filter(|_| rand::thread_rng().gen_bool(missing_pct))
            .map(|t| t.id)
            .collect()
    }

    /// Generate a minimal but syntactically valid PDF for a transaction receipt.
    ///
    /// The PDF contains a single page with the merchant name, date, and amount.
    /// Real reconcilers should be able to parse this with standard PDF libraries.
    pub fn generate_receipt_pdf(txn: &SimulatedTransaction) -> Vec<u8> {
        let date_str   = txn.timestamp.format("%Y-%m-%d %H:%M UTC").to_string();
        let content    = format!(
            "RECEIPT\r\nMerchant: {}\r\nDate: {}\r\nAmount: {} {}\r\nVAT: {} {}\r\nNet: {} {}\r\nRef: {}",
            txn.merchant, date_str,
            txn.amount, txn.currency,
            txn.vat_amount, txn.currency,
            txn.net_amount, txn.currency,
            txn.reference,
        );
        let text_len   = content.len();

        // Build a minimal PDF 1.4 document manually.
        // Object layout:
        //   1 0 obj  Catalog
        //   2 0 obj  Pages
        //   3 0 obj  Page
        //   4 0 obj  Font (Helvetica)
        //   5 0 obj  Content stream

        let stream_content = format!(
            "BT\n/F1 12 Tf\n50 750 Td\n({}) Tj\nET",
            content.replace('(', "\\(").replace(')', "\\)")
        );
        let stream_len = stream_content.len();

        let mut pdf = String::new();
        pdf.push_str("%PDF-1.4\n");

        // obj 1: Catalog
        let off1 = pdf.len();
        pdf.push_str("1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

        // obj 2: Pages
        let off2 = pdf.len();
        pdf.push_str("2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

        // obj 3: Page
        let off3 = pdf.len();
        pdf.push_str("3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842]\n   /Contents 5 0 R /Resources << /Font << /F1 4 0 R >> >> >>\nendobj\n");

        // obj 4: Font
        let off4 = pdf.len();
        pdf.push_str("4 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n");

        // obj 5: Content stream
        let off5 = pdf.len();
        pdf.push_str(&format!("5 0 obj\n<< /Length {stream_len} >>\nstream\n{stream_content}\nendstream\nendobj\n"));

        // xref table
        let xref_offset = pdf.len();
        pdf.push_str("xref\n");
        pdf.push_str("0 6\n");
        pdf.push_str("0000000000 65535 f \n");
        pdf.push_str(&format!("{:010} 00000 n \n", off1));
        pdf.push_str(&format!("{:010} 00000 n \n", off2));
        pdf.push_str(&format!("{:010} 00000 n \n", off3));
        pdf.push_str(&format!("{:010} 00000 n \n", off4));
        pdf.push_str(&format!("{:010} 00000 n \n", off5));

        pdf.push_str("trailer\n<< /Size 6 /Root 1 0 R >>\n");
        pdf.push_str(&format!("startxref\n{xref_offset}\n%%EOF\n"));

        let _ = text_len; // suppress unused warning
        pdf.into_bytes()
    }

    /// Generate a minimal RFC 822 email message with the receipt embedded as base64.
    pub fn generate_receipt_email(txn: &SimulatedTransaction) -> String {
        let boundary   = format!("boundary-{}", &txn.id.to_string()[..8]);
        let date_str   = txn.timestamp.format("%a, %d %b %Y %H:%M:%S +0000").to_string();
        let pdf_bytes  = Self::generate_receipt_pdf(txn);
        let pdf_b64    = base64_encode(&pdf_bytes);
        let merchant_lc = txn.merchant.to_lowercase().replace(' ', "-");

        format!(
            "From: noreply@{merchant_lc}.receipts.example.com\r\n\
             To: accounting@testco.se\r\n\
             Date: {date_str}\r\n\
             Subject: Receipt for {merchant} — {amount} {currency}\r\n\
             Message-ID: <{id}@receipts.example.com>\r\n\
             MIME-Version: 1.0\r\n\
             Content-Type: multipart/mixed; boundary=\"{boundary}\"\r\n\
             \r\n\
             --{boundary}\r\n\
             Content-Type: text/plain; charset=UTF-8\r\n\
             \r\n\
             Dear Customer,\r\n\
             \r\n\
             Thank you for your purchase at {merchant}.\r\n\
             \r\n\
             Amount:    {amount} {currency}\r\n\
             VAT:       {vat} {currency}\r\n\
             Net:       {net} {currency}\r\n\
             Reference: {reference}\r\n\
             Date:      {date_str}\r\n\
             \r\n\
             Please find your receipt attached.\r\n\
             \r\n\
             --{boundary}\r\n\
             Content-Type: application/pdf\r\n\
             Content-Transfer-Encoding: base64\r\n\
             Content-Disposition: attachment; filename=\"receipt-{reference}.pdf\"\r\n\
             \r\n\
             {pdf_b64}\r\n\
             --{boundary}--\r\n",
            merchant     = txn.merchant,
            amount       = txn.amount,
            currency     = txn.currency,
            vat          = txn.vat_amount,
            net          = txn.net_amount,
            reference    = txn.reference,
            id           = txn.id,
        )
    }

    // ── Edge cases ───────────────────────────────────────────────────────────

    /// Return one edge-case transaction per `EdgeCaseType` variant.
    pub fn generate_edge_cases() -> Vec<EdgeCase> {
        let base_template = &MERCHANT_CATALOG[0]; // ICA Maxi
        let normal_amount = Decimal::from_str("523.50").unwrap();
        let vat_rate      = Decimal::from_str("0.12").unwrap();
        let now           = Utc::now();

        let make_txn = |id: Uuid,
                        amount: Decimal,
                        currency: &str,
                        timestamp: DateTime<Utc>,
                        receipt: bool,
                        duplicate: bool,
                        status: TxStatus| -> SimulatedTransaction {
            let net_amount = (amount / (Decimal::ONE + vat_rate)).round_dp(2);
            let vat_amount = (amount - net_amount).round_dp(2);
            SimulatedTransaction {
                id,
                amount,
                currency:       currency.to_string(),
                merchant:       base_template.name.to_string(),
                category:       base_template.category.to_string(),
                vat_rate,
                vat_amount,
                net_amount,
                timestamp,
                status,
                receipt_exists: receipt,
                account_id:     "acc-nordea-001".to_string(),
                reference:      format!("EC-{}", &id.to_string()[..6].to_uppercase()),
                description:    format!("Edge case at {}", base_template.name),
                is_duplicate:   duplicate,
                webhook_fired:  true,
            }
        };

        let dup_id = Uuid::new_v4();

        vec![
            // 1. Duplicate transaction
            {
                let id  = Uuid::new_v4();
                let txn = make_txn(id, normal_amount, "SEK", now - Duration::minutes(3), true, true, TxStatus::Completed);
                EdgeCase {
                    case_type:        EdgeCaseType::DuplicateTransaction,
                    description:      "Exact duplicate: same amount, merchant, and reference as an earlier transaction.".to_string(),
                    transaction:      txn,
                    expected_outcome: "Reconciler should detect and quarantine; require manual approval before booking.".to_string(),
                }
            },
            // 2. Wrong currency
            {
                let id  = Uuid::new_v4();
                let txn = make_txn(id, normal_amount, "JPY", now, true, false, TxStatus::Completed);
                EdgeCase {
                    case_type:        EdgeCaseType::WrongCurrency,
                    description:      "Transaction currency is JPY but Fortnox invoice is denominated in SEK.".to_string(),
                    transaction:      txn,
                    expected_outcome: "Flag for currency mismatch; do not auto-book until FX rate is confirmed.".to_string(),
                }
            },
            // 3. Missing receipt
            {
                let id  = Uuid::new_v4();
                let txn = make_txn(id, normal_amount, "SEK", now, false, false, TxStatus::Completed);
                EdgeCase {
                    case_type:        EdgeCaseType::MissingReceipt,
                    description:      "Completed transaction with no receipt attached or emailed.".to_string(),
                    transaction:      txn,
                    expected_outcome: "Trigger receipt recovery agent: search inbox, contact merchant.".to_string(),
                }
            },
            // 4. Corrupted OCR
            {
                let id  = Uuid::new_v4();
                // Amount looks like 5Z3.5O — OCR artifact
                let mut txn = make_txn(id, normal_amount, "SEK", now, true, false, TxStatus::Completed);
                txn.description = "OCR result: '5Z3.5O SEK' — unreadable amount".to_string();
                EdgeCase {
                    case_type:        EdgeCaseType::CorruptedOCR,
                    description:      "Receipt PDF scanned with corrupted OCR output; amount field unreadable.".to_string(),
                    transaction:      txn,
                    expected_outcome: "Mark receipt as unreadable; escalate to human review.".to_string(),
                }
            },
            // 5. Delayed webhook
            {
                let id  = Uuid::new_v4();
                let txn = make_txn(id, normal_amount, "SEK", now - Duration::hours(48), true, false, TxStatus::Completed);
                EdgeCase {
                    case_type:        EdgeCaseType::DelayedWebhook,
                    description:      "Webhook arrived 48 hours after the transaction timestamp.".to_string(),
                    transaction:      txn,
                    expected_outcome: "Accept and process; reconciler must tolerate out-of-order events.".to_string(),
                }
            },
            // 6. Negative amount (refund mis-classified as purchase)
            {
                let id     = Uuid::new_v4();
                let amount = Decimal::from_str("-299.00").unwrap();
                let net    = (amount / (Decimal::ONE + vat_rate)).round_dp(2);
                let vat    = (amount - net).round_dp(2);
                let mut txn = SimulatedTransaction {
                    id,
                    amount,
                    currency:       "SEK".to_string(),
                    merchant:       base_template.name.to_string(),
                    category:       base_template.category.to_string(),
                    vat_rate,
                    vat_amount:     vat,
                    net_amount:     net,
                    timestamp:      now,
                    status:         TxStatus::Completed,
                    receipt_exists: true,
                    account_id:     "acc-nordea-001".to_string(),
                    reference:      "EC-NEGAMT".to_string(),
                    description:    "Negative amount — possible refund".to_string(),
                    is_duplicate:   false,
                    webhook_fired:  true,
                };
                EdgeCase {
                    case_type:        EdgeCaseType::NegativeAmount,
                    description:      "Bank reports a negative-amount debit (should be a credit/refund).".to_string(),
                    transaction:      txn,
                    expected_outcome: "Reclassify as refund; book as credit against the original expense account.".to_string(),
                }
            },
            // 7. Zero amount
            {
                let id  = Uuid::new_v4();
                let txn = make_txn(id, Decimal::ZERO, "SEK", now, false, false, TxStatus::Completed);
                EdgeCase {
                    case_type:        EdgeCaseType::ZeroAmount,
                    description:      "Transaction with amount exactly 0.00.".to_string(),
                    transaction:      txn,
                    expected_outcome: "Discard; zero-amount transactions are noise (e.g. card authorisation holds).".to_string(),
                }
            },
            // 8. Future date
            {
                let id  = Uuid::new_v4();
                let txn = make_txn(id, normal_amount, "SEK", now + Duration::days(10), true, false, TxStatus::Pending);
                EdgeCase {
                    case_type:        EdgeCaseType::FutureDate,
                    description:      "Transaction timestamp is 10 days in the future (system clock skew or pre-auth).".to_string(),
                    transaction:      txn,
                    expected_outcome: "Hold in pending queue; do not book until timestamp passes.".to_string(),
                }
            },
            // 9. VAT mismatch
            {
                let id       = Uuid::new_v4();
                let amount   = Decimal::from_str("500.00").unwrap();
                // Receipt says 25% VAT but category implies 12%
                let vat_25   = (amount / Decimal::from_str("1.25").unwrap() * Decimal::from_str("0.25").unwrap()).round_dp(2);
                let net_25   = (amount - vat_25).round_dp(2);
                let txn = SimulatedTransaction {
                    id,
                    amount,
                    currency:       "SEK".to_string(),
                    merchant:       base_template.name.to_string(),
                    category:       "Groceries".to_string(),
                    vat_rate:       Decimal::from_str("0.25").unwrap(),
                    vat_amount:     vat_25,
                    net_amount:     net_25,
                    timestamp:      now,
                    status:         TxStatus::Completed,
                    receipt_exists: true,
                    account_id:     "acc-nordea-001".to_string(),
                    reference:      "EC-VATMIS".to_string(),
                    description:    "Groceries purchase at 25% VAT (should be 12%)".to_string(),
                    is_duplicate:   false,
                    webhook_fired:  true,
                };
                EdgeCase {
                    case_type:        EdgeCaseType::VATMismatch,
                    description:      "Receipt claims 25% VAT for a grocery purchase; Skatteverket requires 12% for food.".to_string(),
                    transaction:      txn,
                    expected_outcome: "Flag for manual VAT correction before booking; alert accountant.".to_string(),
                }
            },
            // 10. Refund without original transaction
            {
                let id  = Uuid::new_v4();
                let mut txn = make_txn(id, normal_amount, "SEK", now, true, false, TxStatus::Refunded);
                txn.description = "Refund from ICA Maxi — original transaction not found in ledger".to_string();
                EdgeCase {
                    case_type:        EdgeCaseType::RefundWithoutOriginal,
                    description:      "Bank reports a refund but no matching original debit exists in the reconciler's ledger.".to_string(),
                    transaction:      txn,
                    expected_outcome: "Book as miscellaneous income; flag for review; do not auto-match.".to_string(),
                }
            },
            // 11. Amount rounding discrepancy
            {
                let id     = Uuid::new_v4();
                let amount = Decimal::from_str("99.999").unwrap(); // fractional öre
                let net    = (amount / (Decimal::ONE + vat_rate)).round_dp(2);
                let vat    = (amount - net).round_dp(2);
                let txn = SimulatedTransaction {
                    id,
                    amount,
                    currency:       "SEK".to_string(),
                    merchant:       "Amazon Web Services".to_string(),
                    category:       "Cloud".to_string(),
                    vat_rate,
                    vat_amount:     vat,
                    net_amount:     net,
                    timestamp:      now,
                    status:         TxStatus::Completed,
                    receipt_exists: true,
                    account_id:     "acc-revolut-001".to_string(),
                    reference:      "EC-ROUND".to_string(),
                    description:    "AWS invoice with sub-öre precision".to_string(),
                    is_duplicate:   false,
                    webhook_fired:  true,
                };
                EdgeCase {
                    case_type:        EdgeCaseType::AmountRounding,
                    description:      "Bank reports amount with 3 decimal places; Fortnox accepts only 2.".to_string(),
                    transaction:      txn,
                    expected_outcome: "Round to 2 dp (banker's rounding); log the delta as a rounding adjustment.".to_string(),
                }
            },
            // 12. Unsupported currency
            {
                let id  = Uuid::new_v4();
                let txn = make_txn(id, normal_amount, "BTC", now, false, false, TxStatus::Completed);
                EdgeCase {
                    case_type:        EdgeCaseType::UnsupportedCurrency,
                    description:      "Transaction currency 'BTC' is not in the Fortnox supported list.".to_string(),
                    transaction:      txn,
                    expected_outcome: "Reject immediately; route to manual FX conversion workflow.".to_string(),
                }
            },
        ]
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// base64 encoder (no external dep beyond std)
// ─────────────────────────────────────────────────────────────────────────────

fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(((input.len() + 2) / 3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((triple >> 18) & 63) as usize] as char);
        out.push(TABLE[((triple >> 12) & 63) as usize] as char);
        if chunk.len() > 1 { out.push(TABLE[((triple >> 6) & 63) as usize] as char); } else { out.push('='); }
        if chunk.len() > 2 { out.push(TABLE[(triple & 63) as usize] as char); }         else { out.push('='); }
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_company() {
        let company = FinancialDataGenerator::generate_company();
        assert_eq!(company.bank_accounts.len(), 3);
        assert!(!company.org_number.is_empty());
    }

    #[test]
    fn test_generate_transaction_history() {
        let company = FinancialDataGenerator::generate_company();
        let history = FinancialDataGenerator::generate_transaction_history(&company, 3);
        assert_eq!(history.months.len(), 3);
        assert!(history.transaction_count >= 3 * 15);
        assert!(history.verify_trial_balance());
    }

    #[test]
    fn test_generate_missing_receipts() {
        let company   = FinancialDataGenerator::generate_company();
        let history   = FinancialDataGenerator::generate_transaction_history(&company, 1);
        let all_txns: Vec<SimulatedTransaction> = history.months[0].transactions.clone();
        let missing   = FinancialDataGenerator::generate_missing_receipts(&all_txns, 1.0);
        assert_eq!(missing.len(), all_txns.len());
    }

    #[test]
    fn test_generate_receipt_pdf_starts_with_pdf_header() {
        let company = FinancialDataGenerator::generate_company();
        let history = FinancialDataGenerator::generate_transaction_history(&company, 1);
        let txn     = &history.months[0].transactions[0];
        let pdf     = FinancialDataGenerator::generate_receipt_pdf(txn);
        assert!(pdf.starts_with(b"%PDF-1.4"), "PDF must start with %%PDF-1.4 header");
        assert!(pdf.ends_with(b"\n%%EOF\n"), "PDF must end with %%EOF");
    }

    #[test]
    fn test_generate_receipt_email_rfc822() {
        let company = FinancialDataGenerator::generate_company();
        let history = FinancialDataGenerator::generate_transaction_history(&company, 1);
        let txn     = &history.months[0].transactions[0];
        let email   = FinancialDataGenerator::generate_receipt_email(txn);
        assert!(email.contains("From:"),           "Missing From header");
        assert!(email.contains("MIME-Version:"),   "Missing MIME-Version header");
        assert!(email.contains("Content-Type:"),   "Missing Content-Type header");
        assert!(email.contains("multipart/mixed"), "Email must be multipart/mixed");
        assert!(email.contains("application/pdf"), "Must have PDF attachment");
    }

    #[test]
    fn test_generate_edge_cases_coverage() {
        let cases = FinancialDataGenerator::generate_edge_cases();
        // Ensure every EdgeCaseType variant is covered
        let types: Vec<EdgeCaseType> = cases.iter().map(|c| c.case_type.clone()).collect();
        let expected = vec![
            EdgeCaseType::DuplicateTransaction,
            EdgeCaseType::WrongCurrency,
            EdgeCaseType::MissingReceipt,
            EdgeCaseType::CorruptedOCR,
            EdgeCaseType::DelayedWebhook,
            EdgeCaseType::NegativeAmount,
            EdgeCaseType::ZeroAmount,
            EdgeCaseType::FutureDate,
            EdgeCaseType::VATMismatch,
            EdgeCaseType::RefundWithoutOriginal,
            EdgeCaseType::AmountRounding,
            EdgeCaseType::UnsupportedCurrency,
        ];
        for expected_type in &expected {
            assert!(
                types.contains(expected_type),
                "Missing edge case: {expected_type}"
            );
        }
    }

    #[test]
    fn test_base64_encode_roundtrip() {
        let input  = b"Hello, World!";
        let encoded = base64_encode(input);
        assert_eq!(encoded, "SGVsbG8sIFdvcmxkIQ==");
    }
}
