use anyhow::{anyhow, Context};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

use super::schema::VerifiedReceipt;

#[derive(Debug)]
pub struct VerificationResult {
    pub is_valid: bool,
    pub signature_valid: bool,
    pub hash_matches: bool,
    pub schema_valid: bool,
    pub issuer: String,
    pub error: Option<String>,
}

pub struct ReceiptVerifier;

impl ReceiptVerifier {
    /// Verifiera receipt mot given public key.
    pub fn verify(receipt: &VerifiedReceipt, public_key: &VerifyingKey) -> VerificationResult {
        let issuer = receipt.issuer.name.clone();

        // Schema validation
        let schema_valid = Self::validate_schema(receipt);
        if !schema_valid {
            return VerificationResult {
                is_valid: false,
                signature_valid: false,
                hash_matches: false,
                schema_valid: false,
                issuer,
                error: Some("Schema validation failed: missing required fields or version mismatch".to_string()),
            };
        }

        // Proof must exist for full verification
        let proof = match &receipt.proof {
            Some(p) => p,
            None => {
                return VerificationResult {
                    is_valid: false,
                    signature_valid: false,
                    hash_matches: false,
                    schema_valid: true,
                    issuer,
                    error: Some("No cryptographic proof present".to_string()),
                }
            }
        };

        // Recompute canonical hash (without proof field)
        let computed_hash = super::proof::receipt_canonical_hash(receipt);

        // Check hash matches what was recorded in proof
        let hash_matches = computed_hash == proof.canonical_hash;

        if !hash_matches {
            return VerificationResult {
                is_valid: false,
                signature_valid: false,
                hash_matches: false,
                schema_valid: true,
                issuer,
                error: Some(format!(
                    "Canonical hash mismatch: computed={} stored={}",
                    computed_hash, proof.canonical_hash
                )),
            };
        }

        // Decode and verify signature
        let sig_result = Self::verify_signature(proof, &computed_hash, public_key);
        match sig_result {
            Ok(()) => VerificationResult {
                is_valid: true,
                signature_valid: true,
                hash_matches: true,
                schema_valid: true,
                issuer,
                error: None,
            },
            Err(e) => VerificationResult {
                is_valid: false,
                signature_valid: false,
                hash_matches: true,
                schema_valid: true,
                issuer,
                error: Some(format!("Signature verification failed: {}", e)),
            },
        }
    }

    /// Verifiera utan signature check (bara hash + schema)
    pub fn verify_structure(receipt: &VerifiedReceipt) -> VerificationResult {
        let issuer = receipt.issuer.name.clone();

        let schema_valid = Self::validate_schema(receipt);
        if !schema_valid {
            return VerificationResult {
                is_valid: false,
                signature_valid: false,
                hash_matches: false,
                schema_valid: false,
                issuer,
                error: Some("Schema validation failed".to_string()),
            };
        }

        let proof = match &receipt.proof {
            Some(p) => p,
            None => {
                // No proof – structure is valid but unverified
                return VerificationResult {
                    is_valid: true,
                    signature_valid: false,
                    hash_matches: false,
                    schema_valid: true,
                    issuer,
                    error: Some("No proof present; structure only".to_string()),
                };
            }
        };

        let computed_hash = super::proof::receipt_canonical_hash(receipt);
        let hash_matches = computed_hash == proof.canonical_hash;

        VerificationResult {
            is_valid: hash_matches,
            signature_valid: false, // not checked
            hash_matches,
            schema_valid: true,
            issuer,
            error: if hash_matches {
                None
            } else {
                Some(format!(
                    "Hash mismatch: computed={} stored={}",
                    computed_hash, proof.canonical_hash
                ))
            },
        }
    }

    fn validate_schema(receipt: &VerifiedReceipt) -> bool {
        // VRF version must be present and supported
        if receipt.vrf_version.is_empty() {
            return false;
        }
        // Must have at least major version 1
        let major = receipt
            .vrf_version
            .split('.')
            .next()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        if major < 1 {
            return false;
        }
        // Issuer name must be non-empty
        if receipt.issuer.name.is_empty() {
            return false;
        }
        // Country must be non-empty
        if receipt.issuer.country.is_empty() {
            return false;
        }
        // Currency must be non-empty
        if receipt.totals.currency.is_empty() {
            return false;
        }
        // Payment rail must be present
        if receipt.transaction.payment_rail.is_empty() {
            return false;
        }
        true
    }

    fn verify_signature(
        proof: &super::schema::CryptographicProof,
        hash_hex: &str,
        public_key: &VerifyingKey,
    ) -> anyhow::Result<()> {
        // Decode base64 signature
        let sig_bytes = B64
            .decode(&proof.signature)
            .context("failed to decode base64 signature")?;

        let sig_array: [u8; 64] = sig_bytes
            .try_into()
            .map_err(|_| anyhow!("signature must be 64 bytes, got wrong length"))?;

        let signature = Signature::from_bytes(&sig_array);

        // Decode hash hex to bytes (what was originally signed)
        let hash_bytes = hex_decode(hash_hex).context("failed to decode hash hex")?;

        // Verify
        public_key
            .verify(&hash_bytes, &signature)
            .context("Ed25519 signature verification failed")?;

        Ok(())
    }
}

/// Decode hex string to bytes
fn hex_decode(s: &str) -> anyhow::Result<Vec<u8>> {
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
