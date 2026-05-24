use anyhow::{Context, Result};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use chrono::Utc;
use ed25519_dalek::{Signer, SigningKey};
use rand_core::OsRng;

use super::schema::{CryptographicProof, VerifiedReceipt};

pub struct ReceiptSigner {
    signing_key: SigningKey,
    verification_method: String,
}

impl ReceiptSigner {
    /// Skapa ny signer från random key (för tests)
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        Self {
            signing_key,
            verification_method: "did:web:kvittovalvet.se#key-1".to_string(),
        }
    }

    /// Ladda från raw 32-byte seed
    pub fn from_secret_bytes(bytes: [u8; 32], verification_method: impl Into<String>) -> Self {
        let signing_key = SigningKey::from_bytes(&bytes);
        Self {
            signing_key,
            verification_method: verification_method.into(),
        }
    }

    /// Default test-signer (för Kvittovalvet) med fixed seed för deterministisk test
    pub fn default_test() -> Self {
        Self::from_secret_bytes([42u8; 32], "did:web:kvittovalvet.se#test-key-1")
    }

    pub fn verifying_key(&self) -> ed25519_dalek::VerifyingKey {
        self.signing_key.verifying_key()
    }

    /// Signera ett kvitto. Sätter receipt.proof.
    pub fn sign(&self, receipt: &mut VerifiedReceipt) -> Result<()> {
        // Clear any existing proof before hashing
        receipt.proof = None;

        // Compute canonical hash of receipt without proof
        let hash = super::proof::receipt_canonical_hash(receipt);

        // Sign the hash bytes
        let hash_bytes = hex_decode(&hash).context("failed to decode canonical hash hex")?;
        let signature = self.signing_key.sign(&hash_bytes);
        let sig_b64 = B64.encode(signature.to_bytes());

        receipt.proof = Some(CryptographicProof {
            proof_type: "Ed25519Signature2020".to_string(),
            created: Utc::now(),
            verification_method: self.verification_method.clone(),
            signature: sig_b64,
            canonical_hash: hash,
        });

        Ok(())
    }
}

/// Decode hex string to bytes
fn hex_decode(s: &str) -> Result<Vec<u8>> {
    if s.len() % 2 != 0 {
        anyhow::bail!("hex string has odd length");
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .with_context(|| format!("invalid hex byte at position {}", i))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::receipt_format::verifier::ReceiptVerifier;

    fn make_test_receipt() -> VerifiedReceipt {
        use rust_decimal_macros::dec;
        use uuid::Uuid;
        use crate::receipt_format::schema::*;

        VerifiedReceipt {
            vrf_version: "1.0.0".to_string(),
            receipt_id: Uuid::new_v4(),
            issuer: ReceiptIssuer {
                name: "ICA Maxi".to_string(),
                merchant_id: Some("ICA".to_string()),
                org_number: Some("559141-7042".to_string()),
                vat_number: Some("SE5591417042 01".to_string()),
                country: "SE".to_string(),
                address: None,
                website: None,
                email: None,
                store_id: Some("ICA MAXI 4392".to_string()),
            },
            issued_at: Utc::now(),
            transaction: ReceiptTransaction {
                merchant_reference: Some("KV-20240524-001".to_string()),
                external_id: None,
                payment_rail: "card".to_string(),
                card_last4: Some("4242".to_string()),
                psp_reference: None,
            },
            items: vec![ReceiptLine {
                line_no: 1,
                description: "Mjölk 3% 1L".to_string(),
                sku: Some("7310865084617".to_string()),
                quantity: dec!(2),
                unit: Some("st".to_string()),
                unit_price: dec!(12.90),
                total: dec!(25.80),
                vat_rate: Some(dec!(0.12)),
                vat_amount: Some(dec!(2.76)),
                category: None,
            }],
            totals: ReceiptTotals {
                subtotal: dec!(23.04),
                total_vat: dec!(2.76),
                total: dec!(25.80),
                currency: "SEK".to_string(),
                rounding: dec!(0),
                discount: dec!(0),
            },
            vat: vec![crate::receipt_format::schema::VatRow {
                rate: dec!(0.12),
                base: dec!(23.04),
                vat_amount: dec!(2.76),
                vat_code: Some("S".to_string()),
            }],
            payment: PaymentInfo {
                paid_at: Utc::now(),
                amount: dec!(25.80),
                currency: "SEK".to_string(),
                method: "card".to_string(),
                status: "captured".to_string(),
            },
            metadata: serde_json::Value::Null,
            proof: None,
        }
    }

    #[test]
    fn test_sign_and_verify() {
        let signer = ReceiptSigner::default_test();
        let verifying_key = signer.verifying_key();

        let mut receipt = make_test_receipt();
        signer.sign(&mut receipt).expect("sign failed");

        assert!(receipt.proof.is_some());
        let result = ReceiptVerifier::verify(&receipt, &verifying_key);
        assert!(result.is_valid, "verification failed: {:?}", result.error);
        assert!(result.signature_valid);
        assert!(result.hash_matches);
    }

    #[test]
    fn test_tamper_detection() {
        let signer = ReceiptSigner::default_test();
        let verifying_key = signer.verifying_key();

        let mut receipt = make_test_receipt();
        signer.sign(&mut receipt).expect("sign failed");

        // Tamper with amount
        receipt.totals.total = rust_decimal_macros::dec!(0.01);

        let result = ReceiptVerifier::verify(&receipt, &verifying_key);
        assert!(!result.is_valid, "should have detected tampering");
    }
}
