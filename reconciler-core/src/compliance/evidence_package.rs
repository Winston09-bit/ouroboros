use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use zip::{write::FileOptions, ZipWriter};

use super::chain_of_custody::CustodyEvent;
use super::failure_certificate::FailureCertificate;

// ---------------------------------------------------------------------------
// EvidenceStatus
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EvidenceStatus {
    Verified,
    Partial,
    Missing,
    Unrecoverable,
}

impl std::fmt::Display for EvidenceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvidenceStatus::Verified => write!(f, "verified"),
            EvidenceStatus::Partial => write!(f, "partial"),
            EvidenceStatus::Missing => write!(f, "missing"),
            EvidenceStatus::Unrecoverable => write!(f, "unrecoverable"),
        }
    }
}

// ---------------------------------------------------------------------------
// EvidenceFile
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceFile {
    pub filename: String,
    pub mime_type: String,
    #[serde(skip)]
    pub data: Vec<u8>,
    pub sha256: String,
    pub source: String,
}

impl EvidenceFile {
    pub fn new(filename: String, mime_type: String, data: Vec<u8>, source: String) -> Self {
        let sha256 = {
            let mut hasher = Sha256::new();
            hasher.update(&data);
            format!("{:x}", hasher.finalize())
        };
        EvidenceFile {
            filename,
            mime_type,
            data,
            sha256,
            source,
        }
    }
}

// ---------------------------------------------------------------------------
// EvidenceTransaction
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceTransaction {
    pub transaction_id: Uuid,
    pub amount: Decimal,
    pub currency: String,
    pub date: DateTime<Utc>,
    pub merchant: String,
    pub status: EvidenceStatus,
    pub files: Vec<EvidenceFile>,
    pub chain_of_custody: Vec<CustodyEvent>,
    pub failure_certificate: Option<FailureCertificate>,
}

impl EvidenceTransaction {
    pub fn new(
        transaction_id: Uuid,
        amount: Decimal,
        currency: String,
        date: DateTime<Utc>,
        merchant: String,
    ) -> Self {
        EvidenceTransaction {
            transaction_id,
            amount,
            currency,
            date,
            merchant,
            status: EvidenceStatus::Missing,
            files: Vec::new(),
            chain_of_custody: Vec::new(),
            failure_certificate: None,
        }
    }

    /// Add a file and update status accordingly.
    pub fn attach_file(&mut self, file: EvidenceFile) {
        self.files.push(file);
        if self.status == EvidenceStatus::Missing {
            self.status = EvidenceStatus::Partial;
        }
    }

    /// Mark as fully verified.
    pub fn mark_verified(&mut self) {
        self.status = EvidenceStatus::Verified;
    }

    /// Mark as unrecoverable and attach failure certificate.
    pub fn mark_unrecoverable(&mut self, cert: FailureCertificate) {
        self.status = EvidenceStatus::Unrecoverable;
        self.failure_certificate = Some(cert);
    }
}

// ---------------------------------------------------------------------------
// PackageManifest
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManifest {
    pub version: String,
    pub total_transactions: usize,
    pub verified: usize,
    pub partial: usize,
    pub missing: usize,
    pub unrecoverable: usize,
    pub generated_at: DateTime<Utc>,
    pub package_hash: String,
}

// ---------------------------------------------------------------------------
// EvidencePackage
// ---------------------------------------------------------------------------

pub struct EvidencePackage {
    pub package_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub company_id: Option<Uuid>,
    pub transactions: Vec<EvidenceTransaction>,
    pub manifest: PackageManifest,
}

impl Default for EvidencePackage {
    fn default() -> Self {
        Self::new()
    }
}

impl EvidencePackage {
    pub fn new() -> Self {
        let package_id = Uuid::new_v4();
        let created_at = Utc::now();

        EvidencePackage {
            package_id,
            created_at,
            company_id: None,
            transactions: Vec::new(),
            manifest: PackageManifest {
                version: "1.0".into(),
                total_transactions: 0,
                verified: 0,
                partial: 0,
                missing: 0,
                unrecoverable: 0,
                generated_at: created_at,
                package_hash: String::new(),
            },
        }
    }

    pub fn with_company(mut self, company_id: Uuid) -> Self {
        self.company_id = Some(company_id);
        self
    }

    pub fn add_transaction(&mut self, txn: EvidenceTransaction) {
        self.transactions.push(txn);
        self.recompute_manifest();
    }

    fn recompute_manifest(&mut self) {
        let verified = self
            .transactions
            .iter()
            .filter(|t| t.status == EvidenceStatus::Verified)
            .count();
        let partial = self
            .transactions
            .iter()
            .filter(|t| t.status == EvidenceStatus::Partial)
            .count();
        let missing = self
            .transactions
            .iter()
            .filter(|t| t.status == EvidenceStatus::Missing)
            .count();
        let unrecoverable = self
            .transactions
            .iter()
            .filter(|t| t.status == EvidenceStatus::Unrecoverable)
            .count();

        // Compute package hash from transaction IDs + statuses
        let mut hasher = Sha256::new();
        for txn in &self.transactions {
            hasher.update(txn.transaction_id.as_bytes());
            hasher.update(txn.status.to_string().as_bytes());
        }
        let package_hash = format!("{:x}", hasher.finalize());

        self.manifest = PackageManifest {
            version: "1.0".into(),
            total_transactions: self.transactions.len(),
            verified,
            partial,
            missing,
            unrecoverable,
            generated_at: Utc::now(),
            package_hash,
        };
    }

    /// Build a ZIP archive in memory containing:
    /// - manifest.json
    /// - transactions/<id>/chain_of_custody.json
    /// - transactions/<id>/failure_certificate.json  (if present)
    /// - transactions/<id>/files/<filename>           (all evidence files)
    pub fn to_zip_bytes(&self) -> anyhow::Result<Vec<u8>> {
        use std::io::Write;

        let buf = Vec::new();
        let cursor = std::io::Cursor::new(buf);
        let mut zip = ZipWriter::new(cursor);
        let options: FileOptions<'static, ()> =
            FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        // manifest.json
        let manifest_json =
            serde_json::to_string_pretty(&self.manifest).unwrap_or_default();
        zip.start_file("manifest.json", options)?;
        zip.write_all(manifest_json.as_bytes())?;

        // package-info.json
        let package_info = serde_json::json!({
            "package_id": self.package_id,
            "created_at": self.created_at,
            "company_id": self.company_id,
            "manifest": self.manifest,
        });
        zip.start_file("package-info.json", options)?;
        zip.write_all(serde_json::to_string_pretty(&package_info)?.as_bytes())?;

        for txn in &self.transactions {
            let prefix = format!("transactions/{}", txn.transaction_id);

            // transaction metadata
            let txn_meta = serde_json::json!({
                "transaction_id": txn.transaction_id,
                "amount": txn.amount.to_string(),
                "currency": txn.currency,
                "date": txn.date,
                "merchant": txn.merchant,
                "status": txn.status,
                "file_count": txn.files.len(),
                "has_failure_certificate": txn.failure_certificate.is_some(),
            });
            zip.start_file(format!("{}/transaction.json", prefix), options)?;
            zip.write_all(serde_json::to_string_pretty(&txn_meta)?.as_bytes())?;

            // chain_of_custody.json
            let chain_json =
                serde_json::to_string_pretty(&txn.chain_of_custody).unwrap_or_default();
            zip.start_file(format!("{}/chain_of_custody.json", prefix), options)?;
            zip.write_all(chain_json.as_bytes())?;

            // failure_certificate.json
            if let Some(cert) = &txn.failure_certificate {
                let cert_json = serde_json::to_string_pretty(&cert).unwrap_or_default();
                zip.start_file(
                    format!("{}/failure_certificate.json", prefix),
                    options,
                )?;
                zip.write_all(cert_json.as_bytes())?;

                // Also include PDF version
                let cert_pdf = cert.to_pdf_bytes();
                zip.start_file(
                    format!("{}/failure_certificate.pdf", prefix),
                    options,
                )?;
                zip.write_all(&cert_pdf)?;
            }

            // evidence files
            for file in &txn.files {
                let file_path = format!("{}/files/{}", prefix, file.filename);
                zip.start_file(file_path, options)?;
                zip.write_all(&file.data)?;

                // Also write a sidecar SHA-256 checksum file
                let checksum_path =
                    format!("{}/files/{}.sha256", prefix, file.filename);
                zip.start_file(checksum_path, options)?;
                zip.write_all(
                    format!("{}  {}\n", file.sha256, file.filename).as_bytes(),
                )?;
            }
        }

        let cursor = zip.finish()?;
        Ok(cursor.into_inner())
    }

    pub fn to_json_report(&self) -> serde_json::Value {
        let transactions: Vec<serde_json::Value> = self
            .transactions
            .iter()
            .map(|txn| {
                serde_json::json!({
                    "transaction_id": txn.transaction_id,
                    "amount": txn.amount.to_string(),
                    "currency": txn.currency,
                    "date": txn.date,
                    "merchant": txn.merchant,
                    "status": txn.status,
                    "evidence_files": txn.files.iter().map(|f| serde_json::json!({
                        "filename": f.filename,
                        "mime_type": f.mime_type,
                        "sha256": f.sha256,
                        "source": f.source,
                        "size_bytes": f.data.len(),
                    })).collect::<Vec<_>>(),
                    "custody_events": txn.chain_of_custody.len(),
                    "failure_certificate": txn.failure_certificate.as_ref().map(|c| serde_json::json!({
                        "certificate_id": c.certificate_id,
                        "issued_at": c.issued_at,
                        "legal_basis": c.legal_basis,
                        "certificate_hash": c.certificate_hash,
                    })),
                })
            })
            .collect();

        serde_json::json!({
            "package_id": self.package_id,
            "created_at": self.created_at,
            "company_id": self.company_id,
            "manifest": self.manifest,
            "transactions": transactions,
        })
    }
}
