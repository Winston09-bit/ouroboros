// stripe_connector.rs — Stripe Webhook Receiver & Event Handler
// Verifies Stripe-Signature, parses events, and maps to Canonical Transaction/Invoice.

use crate::canonical::{CanonicalError, Invoice, InvoiceStatus, LineItem, Money, Transaction, TransactionDirection};
use chrono::{DateTime, TimeZone, Utc};
use hmac::{Hmac, Mac};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::str::FromStr;
use tracing::{debug, info, warn};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

// ─── Public Types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeCharge {
    pub id: String,
    pub amount: i64,           // cents
    pub currency: String,
    pub description: Option<String>,
    pub receipt_url: Option<String>,
    pub created: i64,          // unix timestamp
    pub customer: Option<String>,
    pub invoice: Option<String>,
    pub payment_intent: Option<String>,
    pub status: String,
    pub failure_message: Option<String>,
    pub failure_code: Option<String>,
    pub metadata: Option<std::collections::HashMap<String, String>>,
    pub billing_details: Option<StripeBillingDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeBillingDetails {
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address: Option<StripeAddress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeAddress {
    pub city: Option<String>,
    pub country: Option<String>,
    pub line1: Option<String>,
    pub postal_code: Option<String>,
    pub state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeInvoice {
    pub id: String,
    pub number: Option<String>,
    pub customer: Option<String>,
    pub customer_name: Option<String>,
    pub customer_email: Option<String>,
    pub amount_due: i64,       // cents
    pub amount_paid: i64,
    pub amount_remaining: i64,
    pub currency: String,
    pub status: Option<String>,
    pub due_date: Option<i64>,
    pub created: i64,
    pub period_start: Option<i64>,
    pub period_end: Option<i64>,
    pub subscription: Option<String>,
    pub lines: Option<StripeInvoiceLines>,
    pub hosted_invoice_url: Option<String>,
    pub invoice_pdf: Option<String>,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeInvoiceLines {
    pub data: Vec<StripeInvoiceLineItem>,
    pub has_more: bool,
    pub total_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeInvoiceLineItem {
    pub id: String,
    pub description: Option<String>,
    pub amount: i64,
    pub currency: String,
    pub quantity: Option<i64>,
    pub period: Option<StripePeriod>,
    pub price: Option<StripePrice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripePeriod {
    pub start: i64,
    pub end: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripePrice {
    pub id: String,
    pub unit_amount: Option<i64>,
    pub currency: String,
    pub recurring: Option<StripeRecurring>,
    pub nickname: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeRecurring {
    pub interval: String,
    pub interval_count: u32,
}

// ─── Stripe Events ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum StripeEvent {
    ChargeSucceeded(StripeCharge),
    ChargeFailed(StripeCharge),
    InvoicePaid(StripeInvoice),
    InvoiceCreated(StripeInvoice),
    PaymentIntentSucceeded {
        id: String,
        amount: i64,
        currency: String,
        customer: Option<String>,
        description: Option<String>,
        created: i64,
    },
    PaymentIntentFailed {
        id: String,
        amount: i64,
        currency: String,
        failure_message: Option<String>,
    },
    Unknown {
        event_type: String,
        raw: serde_json::Value,
    },
}

// ─── Raw Stripe webhook envelope ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RawStripeEvent {
    id: String,
    #[serde(rename = "type")]
    event_type: String,
    created: i64,
    data: RawStripeEventData,
    livemode: bool,
    api_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawStripeEventData {
    object: serde_json::Value,
}

// ─── Webhook Handler ──────────────────────────────────────────────────────────

pub struct StripeWebhookHandler {
    pub webhook_secret: String,
}

impl StripeWebhookHandler {
    pub fn new(webhook_secret: impl Into<String>) -> Self {
        Self {
            webhook_secret: webhook_secret.into(),
        }
    }

    // ── Signature Verification ────────────────────────────────────────────────

    /// Verify the `Stripe-Signature` header against the raw payload.
    ///
    /// Stripe's format: `t=<timestamp>,v1=<sig1>[,v1=<sig2>...]`
    /// We verify using HMAC-SHA256 over `{timestamp}.{payload}`.
    pub fn verify_signature(&self, payload: &[u8], signature: &str) -> bool {
        let parts: std::collections::HashMap<&str, Vec<&str>> =
            signature.split(',').fold(Default::default(), |mut map, part| {
                if let Some((k, v)) = part.split_once('=') {
                    map.entry(k).or_default().push(v);
                }
                map
            });

        let timestamp = match parts.get("t").and_then(|v| v.first()) {
            Some(t) => *t,
            None => {
                warn!("Stripe signature missing timestamp");
                return false;
            }
        };

        let signatures = match parts.get("v1") {
            Some(sigs) => sigs.clone(),
            None => {
                warn!("Stripe signature missing v1");
                return false;
            }
        };

        // Check timestamp tolerance (±5 minutes)
        let ts: i64 = match timestamp.parse() {
            Ok(t) => t,
            Err(_) => {
                warn!("Stripe signature timestamp not parseable");
                return false;
            }
        };
        let now = Utc::now().timestamp();
        if (now - ts).abs() > 300 {
            warn!("Stripe signature timestamp out of tolerance: {ts}");
            return false;
        }

        // Compute expected signature
        let signed_payload = format!("{timestamp}.");
        let mut signed = signed_payload.into_bytes();
        signed.extend_from_slice(payload);

        let mut mac = HmacSha256::new_from_slice(self.webhook_secret.as_bytes())
            .expect("HMAC can take any key size");
        mac.update(&signed);
        let expected = hex::encode(mac.finalize().into_bytes());

        // Constant-time comparison across all provided v1 sigs
        let verified = signatures.iter().any(|sig| {
            // Compare byte-by-byte to avoid timing attacks
            if sig.len() != expected.len() {
                return false;
            }
            sig.bytes()
                .zip(expected.bytes())
                .fold(0u8, |acc, (a, b)| acc | (a ^ b))
                == 0
        });

        if !verified {
            warn!("Stripe signature verification FAILED");
        } else {
            debug!("Stripe signature verified OK");
        }

        verified
    }

    // ── Event Parsing ─────────────────────────────────────────────────────────

    /// Parse the raw webhook body into a typed `StripeEvent`.
    pub fn parse_event(&self, payload: &[u8]) -> Result<StripeEvent, CanonicalError> {
        let raw: RawStripeEvent = serde_json::from_slice(payload)
            .map_err(|e| CanonicalError::ParseError(format!("Stripe event parse: {e}")))?;

        info!(
            "Stripe event: type={} id={} livemode={}",
            raw.event_type, raw.id, raw.livemode
        );

        let obj = &raw.data.object;

        let event = match raw.event_type.as_str() {
            "charge.succeeded" => {
                let charge: StripeCharge = serde_json::from_value(obj.clone())
                    .map_err(|e| CanonicalError::ParseError(format!("Stripe charge parse: {e}")))?;
                StripeEvent::ChargeSucceeded(charge)
            }
            "charge.failed" => {
                let charge: StripeCharge = serde_json::from_value(obj.clone())
                    .map_err(|e| CanonicalError::ParseError(format!("Stripe charge parse: {e}")))?;
                StripeEvent::ChargeFailed(charge)
            }
            "invoice.paid" => {
                let invoice: StripeInvoice = serde_json::from_value(obj.clone())
                    .map_err(|e| CanonicalError::ParseError(format!("Stripe invoice parse: {e}")))?;
                StripeEvent::InvoicePaid(invoice)
            }
            "invoice.created" => {
                let invoice: StripeInvoice = serde_json::from_value(obj.clone())
                    .map_err(|e| CanonicalError::ParseError(format!("Stripe invoice parse: {e}")))?;
                StripeEvent::InvoiceCreated(invoice)
            }
            "payment_intent.succeeded" => {
                let id = obj["id"].as_str().unwrap_or("").to_string();
                let amount = obj["amount"].as_i64().unwrap_or(0);
                let currency = obj["currency"].as_str().unwrap_or("usd").to_string();
                let customer = obj["customer"].as_str().map(ToString::to_string);
                let description = obj["description"].as_str().map(ToString::to_string);
                let created = obj["created"].as_i64().unwrap_or(raw.created);
                StripeEvent::PaymentIntentSucceeded {
                    id,
                    amount,
                    currency,
                    customer,
                    description,
                    created,
                }
            }
            "payment_intent.payment_failed" => {
                let id = obj["id"].as_str().unwrap_or("").to_string();
                let amount = obj["amount"].as_i64().unwrap_or(0);
                let currency = obj["currency"].as_str().unwrap_or("usd").to_string();
                let failure_message = obj
                    .pointer("/last_payment_error/message")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);
                StripeEvent::PaymentIntentFailed {
                    id,
                    amount,
                    currency,
                    failure_message,
                }
            }
            other => StripeEvent::Unknown {
                event_type: other.to_string(),
                raw: raw.data.object.clone(),
            },
        };

        Ok(event)
    }

    // ── Canonical Mapping ─────────────────────────────────────────────────────

    /// Convert a Stripe charge to a canonical Transaction.
    pub fn to_transaction(charge: &StripeCharge) -> Transaction {
        let amount = Self::cents_to_decimal(charge.amount, &charge.currency);
        let posted_at = Utc.timestamp_opt(charge.created, 0).single().unwrap_or_else(Utc::now);

        let counterparty_name = charge
            .billing_details
            .as_ref()
            .and_then(|b| b.name.clone());

        Transaction {
            id: Uuid::new_v4().to_string(),
            external_id: charge.id.clone(),
            source: "stripe".into(),
            direction: TransactionDirection::Incoming,
            amount: Money {
                amount,
                currency: charge.currency.to_uppercase(),
            },
            counterparty_id: charge.customer.clone(),
            counterparty_name,
            description: charge.description.clone(),
            posted_at,
            raw: serde_json::to_value(charge).unwrap_or_default(),
        }
    }

    /// Convert a Stripe Invoice to a canonical Invoice.
    pub fn invoice_to_canonical(stripe: &StripeInvoice) -> Invoice {
        let currency = stripe.currency.to_uppercase();
        let amount = Self::cents_to_decimal(stripe.amount_due, &currency);
        let amount_paid = Self::cents_to_decimal(stripe.amount_paid, &currency);

        let status = match stripe.status.as_deref() {
            Some("paid") => InvoiceStatus::Paid,
            Some("open") if stripe.amount_remaining > 0 => InvoiceStatus::Unpaid,
            Some("void") => InvoiceStatus::Cancelled,
            Some("draft") => InvoiceStatus::Draft,
            Some(other) => InvoiceStatus::Unknown(other.to_string()),
            None => InvoiceStatus::Unknown("unknown".into()),
        };

        let due_date = stripe
            .due_date
            .and_then(|ts| Utc.timestamp_opt(ts, 0).single())
            .map(|d| d.format("%Y-%m-%d").to_string());

        let invoice_date = Utc
            .timestamp_opt(stripe.created, 0)
            .single()
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default();

        let line_items: Vec<LineItem> = stripe
            .lines
            .as_ref()
            .map(|lines| {
                lines
                    .data
                    .iter()
                    .map(|item| {
                        let unit_amount = item
                            .price
                            .as_ref()
                            .and_then(|p| p.unit_amount)
                            .unwrap_or(item.amount);
                        LineItem {
                            description: item.description.clone(),
                            quantity: item
                                .quantity
                                .map(|q| Decimal::from(q as i32)),
                            unit_price: Some(Money {
                                amount: Self::cents_to_decimal(unit_amount, &currency),
                                currency: currency.clone(),
                            }),
                            total: Money {
                                amount: Self::cents_to_decimal(item.amount, &currency),
                                currency: currency.clone(),
                            },
                            account_number: None,
                            vat_rate: None,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        Invoice {
            id: Uuid::new_v4().to_string(),
            external_id: stripe.id.clone(),
            source: "stripe".into(),
            invoice_number: stripe.number.clone(),
            counterparty_name: stripe
                .customer_name
                .clone()
                .or_else(|| stripe.customer_email.clone())
                .unwrap_or_default(),
            counterparty_id: stripe.customer.clone(),
            is_outgoing: false, // Stripe invoices are revenue (incoming)
            amount: Money {
                amount,
                currency: currency.clone(),
            },
            vat_amount: None, // Stripe handles tax separately
            status,
            invoice_date,
            due_date,
            line_items,
            raw: serde_json::to_value(stripe).unwrap_or_default(),
        }
    }

    /// Convert a PaymentIntent to a canonical Transaction.
    pub fn payment_intent_to_transaction(
        id: &str,
        amount: i64,
        currency: &str,
        customer: Option<&str>,
        description: Option<&str>,
        created: i64,
    ) -> Transaction {
        let canon_amount = Self::cents_to_decimal(amount, currency);
        let posted_at = Utc.timestamp_opt(created, 0).single().unwrap_or_else(Utc::now);

        Transaction {
            id: Uuid::new_v4().to_string(),
            external_id: id.to_string(),
            source: "stripe".into(),
            direction: TransactionDirection::Incoming,
            amount: Money {
                amount: canon_amount,
                currency: currency.to_uppercase(),
            },
            counterparty_id: customer.map(ToString::to_string),
            counterparty_name: None,
            description: description.map(ToString::to_string),
            posted_at,
            raw: serde_json::json!({
                "type": "payment_intent",
                "id": id,
                "amount": amount,
                "currency": currency
            }),
        }
    }

    // ── Convenience: event → transaction ─────────────────────────────────────

    /// Best-effort extraction of a Transaction from any StripeEvent.
    pub fn event_to_transaction(event: &StripeEvent) -> Option<Transaction> {
        match event {
            StripeEvent::ChargeSucceeded(c) => Some(Self::to_transaction(c)),
            StripeEvent::InvoicePaid(inv) => {
                // Treat paid invoice as an incoming transaction
                let amount =
                    Self::cents_to_decimal(inv.amount_paid, &inv.currency.to_uppercase());
                let posted_at = Utc
                    .timestamp_opt(inv.created, 0)
                    .single()
                    .unwrap_or_else(Utc::now);
                Some(Transaction {
                    id: Uuid::new_v4().to_string(),
                    external_id: inv.id.clone(),
                    source: "stripe".into(),
                    direction: TransactionDirection::Incoming,
                    amount: Money {
                        amount,
                        currency: inv.currency.to_uppercase(),
                    },
                    counterparty_id: inv.customer.clone(),
                    counterparty_name: inv.customer_name.clone(),
                    description: inv
                        .number
                        .as_ref()
                        .map(|n| format!("Invoice {n}")),
                    posted_at,
                    raw: serde_json::to_value(inv).unwrap_or_default(),
                })
            }
            StripeEvent::PaymentIntentSucceeded {
                id,
                amount,
                currency,
                customer,
                description,
                created,
            } => Some(Self::payment_intent_to_transaction(
                id,
                *amount,
                currency,
                customer.as_deref(),
                description.as_deref(),
                *created,
            )),
            _ => None,
        }
    }

    // ── Private Helpers ───────────────────────────────────────────────────────

    /// Convert Stripe integer cents to Decimal, respecting zero-decimal currencies.
    fn cents_to_decimal(amount: i64, currency: &str) -> Decimal {
        // Stripe zero-decimal currencies (no division needed)
        const ZERO_DECIMAL: &[&str] = &[
            "bif", "clp", "gnf", "jpy", "kmf", "krw", "mga", "pyg", "rwf",
            "ugx", "vnd", "vuv", "xaf", "xof", "xpf",
        ];

        let lower = currency.to_lowercase();
        if ZERO_DECIMAL.contains(&lower.as_str()) {
            Decimal::from(amount)
        } else {
            Decimal::new(amount, 2) // divide by 100
        }
    }
}

// ─── Axum Route Integration ───────────────────────────────────────────────────

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use tokio::sync::broadcast;

/// Shared state for the Axum webhook endpoint.
pub struct StripeWebhookState {
    pub handler: StripeWebhookHandler,
    pub event_tx: broadcast::Sender<StripeEvent>,
}

/// POST /webhooks/stripe
pub async fn stripe_webhook_endpoint(
    State(state): State<Arc<StripeWebhookState>>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let signature = match headers
        .get("Stripe-Signature")
        .and_then(|v| v.to_str().ok())
    {
        Some(s) => s.to_string(),
        None => {
            warn!("Stripe webhook missing Stripe-Signature header");
            return (StatusCode::BAD_REQUEST, "Missing Stripe-Signature").into_response();
        }
    };

    if !state.handler.verify_signature(&body, &signature) {
        return (StatusCode::UNAUTHORIZED, "Invalid signature").into_response();
    }

    match state.handler.parse_event(&body) {
        Ok(event) => {
            info!("Stripe webhook processed successfully");
            let _ = state.event_tx.send(event); // ignore if no receivers
            StatusCode::OK.into_response()
        }
        Err(e) => {
            warn!("Stripe webhook parse error: {e}");
            (StatusCode::UNPROCESSABLE_ENTITY, e.to_string()).into_response()
        }
    }
}
