use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// VRF 1.0 – Verified Receipt Format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedReceipt {
    /// Schema version – semver
    pub vrf_version: String,
    /// Unique receipt identifier (UUID v4)
    pub receipt_id: Uuid,
    /// Issuer (merchant) info
    pub issuer: ReceiptIssuer,
    /// Receipt timestamp (issued_at on merchant side)
    pub issued_at: DateTime<Utc>,
    /// Transaction info
    pub transaction: ReceiptTransaction,
    /// Itemized lines
    pub items: Vec<ReceiptLine>,
    /// Total amounts breakdown
    pub totals: ReceiptTotals,
    /// VAT breakdown
    pub vat: Vec<VatRow>,
    /// Payment info
    pub payment: PaymentInfo,
    /// Optional metadata
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub metadata: serde_json::Value,
    /// Cryptographic proof (only present in signed receipts)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proof: Option<CryptographicProof>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptIssuer {
    pub name: String,
    /// "ICA" matchar MerchantResolver
    pub merchant_id: Option<String>,
    /// 559141-7042
    pub org_number: Option<String>,
    /// SE5591417042 01
    pub vat_number: Option<String>,
    pub country: String,
    pub address: Option<String>,
    pub website: Option<String>,
    pub email: Option<String>,
    /// "ICA MAXI 4392"
    pub store_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptTransaction {
    /// ordernummer/kvittonummer
    pub merchant_reference: Option<String>,
    /// bank-transaction-id
    pub external_id: Option<String>,
    /// "card", "swish", "klarna"
    pub payment_rail: String,
    pub card_last4: Option<String>,
    /// Stripe charge_id etc
    pub psp_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptLine {
    pub line_no: u32,
    pub description: String,
    pub sku: Option<String>,
    pub quantity: Decimal,
    /// "st", "kg", "l"
    pub unit: Option<String>,
    pub unit_price: Decimal,
    pub total: Decimal,
    /// 0.25 = 25%
    pub vat_rate: Option<Decimal>,
    pub vat_amount: Option<Decimal>,
    /// EAN-kategori
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptTotals {
    /// excl VAT
    pub subtotal: Decimal,
    pub total_vat: Decimal,
    /// incl VAT
    pub total: Decimal,
    /// "SEK", "EUR"
    pub currency: String,
    /// öresavrundning
    pub rounding: Decimal,
    pub discount: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VatRow {
    /// 0.25, 0.12, 0.06, 0.00
    pub rate: Decimal,
    /// belopp moms beräknas på
    pub base: Decimal,
    pub vat_amount: Decimal,
    /// "S", "Z", "E" (Peppol-koder)
    pub vat_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentInfo {
    pub paid_at: DateTime<Utc>,
    pub amount: Decimal,
    pub currency: String,
    /// "card", "swish", "invoice"
    pub method: String,
    /// "captured", "authorized", "refunded"
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptographicProof {
    /// "Ed25519Signature2020"
    pub proof_type: String,
    pub created: DateTime<Utc>,
    /// "did:web:kvittovalvet.se#key-1" eller URL till public key
    pub verification_method: String,
    /// base64 av signaturen
    pub signature: String,
    /// SHA-256 av canonical JSON (utan proof-fältet)
    pub canonical_hash: String,
}
