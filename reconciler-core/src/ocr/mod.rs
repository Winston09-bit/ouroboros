// src/ocr/mod.rs — OCR Pipeline
// Reconciler OCR + Document Intelligence Pipeline

pub mod parser;
pub mod fraud_detection;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::process::Command;
use std::io::Write;
use tempfile::NamedTempFile;

use parser::FinancialDocumentParser;

// ─────────────────────────────────────────────
// Public types
// ─────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum DocumentType {
    Receipt,
    Invoice,
    BankStatement,
    Unknown,
}

impl std::fmt::Display for DocumentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocumentType::Receipt => write!(f, "Receipt"),
            DocumentType::Invoice => write!(f, "Invoice"),
            DocumentType::BankStatement => write!(f, "BankStatement"),
            DocumentType::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExtractedLineItem {
    pub description: String,
    pub quantity: Option<Decimal>,
    pub unit_price: Option<Decimal>,
    pub total: Decimal,
}

#[derive(Debug, Clone)]
pub struct ExtractedDocumentData {
    pub doc_type: DocumentType,
    pub total_amount: Option<Decimal>,
    pub tax_amount: Option<Decimal>,
    pub tax_rate: Option<Decimal>,
    pub currency: Option<String>,
    pub vendor_name: Option<String>,
    pub vendor_vat: Option<String>,
    pub date: Option<NaiveDate>,
    pub invoice_number: Option<String>,
    pub line_items: Vec<ExtractedLineItem>,
    pub confidence: f64,
}

impl Default for ExtractedDocumentData {
    fn default() -> Self {
        Self {
            doc_type: DocumentType::Unknown,
            total_amount: None,
            tax_amount: None,
            tax_rate: None,
            currency: None,
            vendor_name: None,
            vendor_vat: None,
            date: None,
            invoice_number: None,
            line_items: Vec::new(),
            confidence: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OcrResult {
    pub raw_text: String,
    pub language: String,
    pub confidence: f64,
    pub extracted: ExtractedDocumentData,
}

// ─────────────────────────────────────────────
// OCR Pipeline
// ─────────────────────────────────────────────

pub struct OcrPipeline {
    parser: FinancialDocumentParser,
    /// Minimum confidence score to consider extraction valid
    pub min_confidence: f64,
    /// Whether to run image pre-processing before OCR
    pub preprocess: bool,
}

impl Default for OcrPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl OcrPipeline {
    pub fn new() -> Self {
        Self {
            parser: FinancialDocumentParser::new(),
            min_confidence: 0.3,
            preprocess: true,
        }
    }

    // ── Main entry: process any document (PDF, image, scan) ──────────────────
    /// Dispatch on MIME type and return a fully-structured `OcrResult`.
    pub async fn process(&self, document: &[u8], mime_type: &str) -> OcrResult {
        let raw_text = match mime_type {
            "application/pdf" => self.extract_pdf_text(document).await,
            "image/png" | "image/jpeg" | "image/jpg" | "image/tiff"
            | "image/bmp" | "image/webp" => {
                let preprocessed = if self.preprocess {
                    self.preprocess_image(document)
                } else {
                    document.to_vec()
                };
                self.extract_image_text(&preprocessed).await
            }
            _ => {
                // Attempt image extraction as a fallback
                let preprocessed = if self.preprocess {
                    self.preprocess_image(document)
                } else {
                    document.to_vec()
                };
                self.extract_image_text(&preprocessed).await
            }
        };

        let language = FinancialDocumentParser::detect_language(&raw_text);
        let extracted = self.parser.parse(&raw_text);
        let confidence = FinancialDocumentParser::confidence_score(&extracted);

        OcrResult {
            raw_text,
            language,
            confidence,
            extracted,
        }
    }

    // ── Image pre-processing ─────────────────────────────────────────────────
    /// Pre-process image: deskew, enhance contrast, resize for better OCR.
    ///
    /// Uses ImageMagick (`convert`) if available; otherwise returns the
    /// original bytes unchanged so the pipeline never hard-fails here.
    pub fn preprocess_image(&self, data: &[u8]) -> Vec<u8> {
        // Write input to a temp file
        let mut input_file = match NamedTempFile::new() {
            Ok(f) => f,
            Err(_) => return data.to_vec(),
        };
        if input_file.write_all(data).is_err() {
            return data.to_vec();
        }

        let output_file = match NamedTempFile::new() {
            Ok(f) => f,
            Err(_) => return data.to_vec(),
        };
        let output_path = output_file.path().to_path_buf();
        // Keep the file alive until we read back from it
        drop(output_file);

        // ImageMagick pipeline:
        //  -deskew 40%       → straighten skewed scans
        //  -normalize        → stretch contrast (helps OCR on faded receipts)
        //  -resize 200%      → up-scale small images for Tesseract
        //  -sharpen 0x1      → light sharpening pass
        //  -colorspace Gray  → greyscale is faster + more accurate for text
        let status = Command::new("convert")
            .arg(input_file.path())
            .args([
                "-deskew", "40%",
                "-normalize",
                "-resize", "200%",
                "-sharpen", "0x1",
                "-colorspace", "Gray",
            ])
            .arg(&output_path)
            .status();

        match status {
            Ok(s) if s.success() => {
                std::fs::read(&output_path).unwrap_or_else(|_| data.to_vec())
            }
            _ => data.to_vec(), // graceful fallback
        }
    }

    // ── PDF text extraction ──────────────────────────────────────────────────
    /// Extract text from a PDF using `pdftotext` (poppler-utils).
    ///
    /// Falls back to rendering page-images and running OCR if the PDF is
    /// image-only (scanned) or if `pdftotext` is not installed.
    pub async fn extract_pdf_text(&self, pdf_bytes: &[u8]) -> String {
        let mut input_file = match NamedTempFile::new() {
            Ok(f) => f,
            Err(_) => return String::new(),
        };
        if input_file.write_all(pdf_bytes).is_err() {
            return String::new();
        }

        // Try pdftotext first (fastest, best for digital PDFs)
        let output = Command::new("pdftotext")
            .arg("-layout")    // preserve whitespace layout
            .arg("-enc")
            .arg("UTF-8")
            .arg(input_file.path())
            .arg("-")          // stdout
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout).to_string();
                // If we got meaningful text (> 50 chars of non-whitespace), return it
                if text.chars().filter(|c| !c.is_whitespace()).count() > 50 {
                    return text;
                }
            }
        }

        // Fallback: render PDF pages as images, then OCR each page
        self.ocr_pdf_pages(pdf_bytes, input_file.path()).await
    }

    /// Render PDF pages to images and OCR them individually.
    async fn ocr_pdf_pages(
        &self,
        _pdf_bytes: &[u8],
        pdf_path: &std::path::Path,
    ) -> String {
        // Use pdftoppm to render pages as PPM images
        let tmp_dir = match tempfile::TempDir::new() {
            Ok(d) => d,
            Err(_) => return String::new(),
        };
        let prefix = tmp_dir.path().join("page");

        let status = Command::new("pdftoppm")
            .arg("-r")
            .arg("300") // 300 DPI for quality OCR
            .arg(pdf_path)
            .arg(&prefix)
            .status();

        if status.is_err() || !status.unwrap().success() {
            return String::new();
        }

        let mut full_text = String::new();
        let mut page_files: Vec<_> = std::fs::read_dir(tmp_dir.path())
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| {
                        p.extension()
                            .map(|e| e == "ppm" || e == "png")
                            .unwrap_or(false)
                    })
                    .collect()
            })
            .unwrap_or_default();

        page_files.sort(); // process pages in order

        for page_path in page_files {
            if let Ok(bytes) = std::fs::read(&page_path) {
                let processed = self.preprocess_image(&bytes);
                let page_text = self.extract_image_text(&processed).await;
                if !full_text.is_empty() {
                    full_text.push_str("\n\n--- PAGE BREAK ---\n\n");
                }
                full_text.push_str(&page_text);
            }
        }

        full_text
    }

    // ── Image OCR via Tesseract ──────────────────────────────────────────────
    /// Extract text from an image using the Tesseract CLI.
    ///
    /// Tries `--oem 1` (LSTM) with `--psm 3` (auto page segmentation).
    /// Passes `swe+eng+deu+fra` language data so Swedish/EU docs parse well.
    pub async fn extract_image_text(&self, image_bytes: &[u8]) -> String {
        let mut input_file = match NamedTempFile::with_suffix(".png") {
            Ok(f) => f,
            Err(_) => return String::new(),
        };
        if input_file.write_all(image_bytes).is_err() {
            return String::new();
        }

        let output_file = match NamedTempFile::new() {
            Ok(f) => f,
            Err(_) => return String::new(),
        };
        let output_base = output_file
            .path()
            .to_string_lossy()
            .into_owned();
        drop(output_file); // release so tesseract can write <base>.txt

        // Primary attempt: multi-language LSTM
        let status = Command::new("tesseract")
            .arg(input_file.path())
            .arg(&output_base)
            .args([
                "-l", "swe+eng+deu+fra",
                "--oem", "1",   // LSTM engine
                "--psm", "3",   // fully automatic page segmentation
            ])
            .status();

        if let Ok(s) = status {
            if s.success() {
                let txt_path = format!("{}.txt", output_base);
                if let Ok(text) = std::fs::read_to_string(&txt_path) {
                    let _ = std::fs::remove_file(&txt_path);
                    if !text.trim().is_empty() {
                        return text;
                    }
                }
            }
        }

        // Fallback: English-only if language packs not available
        let status2 = Command::new("tesseract")
            .arg(input_file.path())
            .arg(&output_base)
            .args(["--oem", "1", "--psm", "3"])
            .status();

        if let Ok(s) = status2 {
            if s.success() {
                let txt_path = format!("{}.txt", output_base);
                if let Ok(text) = std::fs::read_to_string(&txt_path) {
                    let _ = std::fs::remove_file(&txt_path);
                    return text;
                }
            }
        }

        String::new()
    }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pipeline() -> OcrPipeline {
        OcrPipeline::new()
    }

    #[test]
    fn test_document_type_display() {
        assert_eq!(DocumentType::Receipt.to_string(), "Receipt");
        assert_eq!(DocumentType::Invoice.to_string(), "Invoice");
        assert_eq!(DocumentType::BankStatement.to_string(), "BankStatement");
        assert_eq!(DocumentType::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn test_default_pipeline_config() {
        let p = make_pipeline();
        assert!(p.preprocess);
        assert!(p.min_confidence > 0.0);
    }

    #[test]
    fn test_extracted_data_defaults() {
        let d = ExtractedDocumentData::default();
        assert_eq!(d.doc_type, DocumentType::Unknown);
        assert!(d.total_amount.is_none());
        assert!(d.line_items.is_empty());
        assert_eq!(d.confidence, 0.0);
    }

    #[tokio::test]
    async fn test_process_unknown_mime_does_not_panic() {
        let pipeline = make_pipeline();
        // Passes empty bytes with unsupported MIME — must not panic
        let result = pipeline.process(b"", "application/octet-stream").await;
        assert!(result.raw_text.is_empty() || !result.raw_text.is_empty());
    }

    #[test]
    fn test_preprocess_image_returns_bytes_on_missing_imagemagick() {
        let pipeline = make_pipeline();
        let dummy = vec![0xFFu8, 0xD8, 0xFF]; // partial JPEG magic bytes
        let out = pipeline.preprocess_image(&dummy);
        // Either processed or original — must never be empty given non-empty input
        assert!(!out.is_empty());
    }
}
pub mod parser;\npub mod fraud_detection;
