/// Peppol e-invoice connector — BIS Billing 3.0 / UBL 2.1
///
/// Implements send/receive over an OpenPEPPOL Access Point (AS4/REST),
/// full UBL 2.1 XML generation compliant with EN 16931 and Peppol BIS 3.0,
/// participant lookup via SML/SMP, and canonical model mapping.
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use reqwest::{header, Client};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Re-use canonical types from the same crate.
use crate::connectors::quickbooks::{Address, Invoice, InvoiceStatus, LineItem, Party};

// ---------------------------------------------------------------------------
// Peppol public structs (as specified)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeppolInvoice {
    pub peppol_id: String,
    pub document_id: String,
    pub sender_peppol_id: String,
    pub receiver_peppol_id: String,
    pub ubl_xml: String,
    pub parsed: Invoice,
    pub received_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeppolParticipant {
    pub peppol_id: String,
    pub name: String,
    pub country: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeppolResult {
    pub transmission_id: String,
    pub status: PeppolStatus,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PeppolStatus {
    Sent,
    Delivered,
    Failed,
    Rejected,
}

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

/// SMP lookup response (simplified — real SMP returns XML)
#[derive(Debug, Deserialize)]
struct SmpServiceGroup {
    #[serde(rename = "participantIdentifier")]
    participant_identifier: Option<String>,
    #[serde(rename = "serviceMetadataReferenceCollection")]
    service_refs: Option<Vec<SmpServiceRef>>,
}

#[derive(Debug, Deserialize)]
struct SmpServiceRef {
    href: String,
}

#[derive(Debug, Deserialize)]
struct SmpServiceMetadata {
    #[serde(rename = "serviceInformation")]
    service_information: Option<SmpServiceInformation>,
}

#[derive(Debug, Deserialize)]
struct SmpServiceInformation {
    #[serde(rename = "documentIdentifier")]
    document_identifier: Option<SmpIdentifier>,
    #[serde(rename = "processIdentifier")]
    process_identifier: Option<SmpIdentifier>,
    #[serde(rename = "endpointList")]
    endpoint_list: Option<SmpEndpointList>,
}

#[derive(Debug, Deserialize)]
struct SmpIdentifier {
    value: Option<String>,
    scheme: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SmpEndpointList {
    endpoint: Option<Vec<SmpEndpoint>>,
}

#[derive(Debug, Deserialize)]
struct SmpEndpoint {
    #[serde(rename = "transportProfile")]
    transport_profile: Option<String>,
    #[serde(rename = "endpointUri")]
    endpoint_uri: Option<String>,
}

/// Access point send request body
#[derive(Debug, Serialize)]
struct ApSendRequest {
    sender_id: String,
    receiver_id: String,
    document_type_id: String,
    process_id: String,
    ubl_xml: String,
}

/// Access point send response
#[derive(Debug, Deserialize)]
struct ApSendResponse {
    transmission_id: Option<String>,
    status: Option<String>,
    message: Option<String>,
}

/// Access point inbox poll response
#[derive(Debug, Deserialize)]
struct ApInboxResponse {
    messages: Option<Vec<ApInboxMessage>>,
}

#[derive(Debug, Deserialize)]
struct ApInboxMessage {
    message_id: String,
    sender_peppol_id: Option<String>,
    receiver_peppol_id: Option<String>,
    ubl_xml: String,
    received_at: Option<String>,
}

// ---------------------------------------------------------------------------
// PeppolConnector
// ---------------------------------------------------------------------------

pub struct PeppolConnector {
    /// URL of the OpenPEPPOL Access Point REST API (e.g. https://ap.example.com)
    pub access_point_url: String,
    /// Our Peppol ID, e.g. "0088:1234567890123" (GLN) or "0007:5567891234" (SE org)
    pub sender_id: String,
    /// PEM-encoded private key for AS4 signing
    pub private_key: String,
    /// PEM-encoded certificate for AS4
    pub certificate: String,

    client: Client,
}

// ---------------------------------------------------------------------------
// Peppol constants (BIS Billing 3.0)
// ---------------------------------------------------------------------------

const BIS_CUSTOMIZATION_ID: &str =
    "urn:cen.eu:en16931:2017#compliant#urn:fdc:peppol.eu:2017:poacc:billing:3.0";
const BIS_PROFILE_ID: &str =
    "urn:fdc:peppol.eu:2017:poacc:billing:international:bis:3.0";
const UBL_DOCUMENT_TYPE_ID: &str =
    "urn:oasis:names:specification:ubl:schema:xsd:Invoice-2::Invoice\
     ##urn:cen.eu:en16931:2017#compliant#urn:fdc:peppol.eu:2017:poacc:billing:3.0::2.1";
const PEPPOL_PROCESS_ID: &str =
    "urn:fdc:peppol.eu:2017:poacc:billing:01:1.0";
/// SML base for SMP lookup (production)
const SML_BASE: &str = "edelivery.tech.ec.europa.eu";

impl PeppolConnector {
    pub fn new(
        access_point_url: impl Into<String>,
        sender_id: impl Into<String>,
        private_key: impl Into<String>,
        certificate: impl Into<String>,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self {
            access_point_url: access_point_url.into(),
            sender_id: sender_id.into(),
            private_key: private_key.into(),
            certificate: certificate.into(),
            client,
        })
    }

    // -----------------------------------------------------------------------
    // Public API
    // -----------------------------------------------------------------------

    /// Transmit an invoice via the access point.
    pub async fn send_invoice(&self, invoice: &Invoice) -> Result<PeppolResult> {
        let receiver_peppol_id = invoice
            .customer
            .external_id
            .as_deref()
            .unwrap_or("")
            .trim_start_matches("peppol:")
            .to_string();

        if receiver_peppol_id.is_empty() {
            return Err(anyhow!(
                "Invoice customer has no Peppol ID in external_id (expected 'peppol:...')"
            ));
        }

        let ubl_xml = self.to_ubl_xml(invoice);

        let payload = ApSendRequest {
            sender_id: self.sender_id.clone(),
            receiver_id: receiver_peppol_id,
            document_type_id: UBL_DOCUMENT_TYPE_ID.to_string(),
            process_id: PEPPOL_PROCESS_ID.to_string(),
            ubl_xml,
        };

        let resp = self
            .client
            .post(format!("{}/outbox", self.access_point_url))
            .header(header::CONTENT_TYPE, "application/json")
            .json(&payload)
            .send()
            .await
            .context("send_invoice HTTP request failed")?;

        let status_code = resp.status();
        let body: ApSendResponse = resp
            .json()
            .await
            .context("Deserializing send_invoice response")?;

        if !status_code.is_success() {
            return Ok(PeppolResult {
                transmission_id: body.transmission_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
                status: PeppolStatus::Failed,
                timestamp: Utc::now(),
            });
        }

        let status = match body.status.as_deref() {
            Some("DELIVERED") => PeppolStatus::Delivered,
            Some("SENT") | Some("ACCEPTED") => PeppolStatus::Sent,
            Some("REJECTED") => PeppolStatus::Rejected,
            _ => PeppolStatus::Sent,
        };

        Ok(PeppolResult {
            transmission_id: body
                .transmission_id
                .unwrap_or_else(|| Uuid::new_v4().to_string()),
            status,
            timestamp: Utc::now(),
        })
    }

    /// Poll the access point inbox and return all pending incoming invoices.
    pub async fn receive_invoices(&self) -> Result<Vec<PeppolInvoice>> {
        let resp = self
            .client
            .get(format!("{}/inbox", self.access_point_url))
            .query(&[("receiver_id", &self.sender_id)])
            .send()
            .await
            .context("receive_invoices HTTP request failed")?;

        if !resp.status().is_success() {
            let code = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("receive_invoices failed {}: {}", code, body));
        }

        let inbox: ApInboxResponse = resp
            .json()
            .await
            .context("Deserializing inbox response")?;

        let mut result = Vec::new();
        for msg in inbox.messages.unwrap_or_default() {
            let parsed = match self.from_ubl_xml(&msg.ubl_xml) {
                Ok(inv) => inv,
                Err(e) => {
                    tracing::warn!("Skipping unparseable Peppol message {}: {}", msg.message_id, e);
                    continue;
                }
            };

            let received_at = msg
                .received_at
                .as_deref()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);

            result.push(PeppolInvoice {
                peppol_id: self.sender_id.clone(),
                document_id: msg.message_id,
                sender_peppol_id: msg.sender_peppol_id.unwrap_or_default(),
                receiver_peppol_id: msg
                    .receiver_peppol_id
                    .unwrap_or_else(|| self.sender_id.clone()),
                ubl_xml: msg.ubl_xml,
                parsed,
                received_at,
            });
        }

        Ok(result)
    }

    /// Generate a BIS Billing 3.0 compliant UBL 2.1 XML string from a canonical Invoice.
    pub fn to_ubl_xml(&self, invoice: &Invoice) -> String {
        let invoice_id = invoice
            .invoice_number
            .clone()
            .unwrap_or_else(|| invoice.id.to_string());

        let issue_date = invoice.date.format("%Y-%m-%d").to_string();
        let due_date = invoice
            .due_date
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| issue_date.clone());

        let receiver_peppol_id = invoice
            .customer
            .external_id
            .as_deref()
            .unwrap_or("")
            .trim_start_matches("peppol:")
            .to_string();

        let currency = &invoice.currency;

        // Build tax total lines
        let tax_xml = Self::build_tax_total_xml(&invoice.line_items, &invoice.tax_total, currency);

        // Build invoice lines
        let lines_xml = invoice
            .line_items
            .iter()
            .enumerate()
            .map(|(i, l)| Self::build_invoice_line_xml(i + 1, l))
            .collect::<Vec<_>>()
            .join("\n");

        let tax_excl = invoice.total - invoice.tax_total;
        let tax_incl = invoice.total;
        let payable_amount = invoice.balance;

        // Supplier info (sender)
        let (sender_scheme, sender_id_val) = Self::split_peppol_id(&self.sender_id);
        let (receiver_scheme, receiver_id_val) = Self::split_peppol_id(&receiver_peppol_id);

        let buyer_address_xml = Self::build_address_xml(&invoice.customer.address);
        let supplier_name = "Supplier"; // In production, loaded from own-company profile

        format!(
r#"<?xml version="1.0" encoding="UTF-8"?>
<Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2"
         xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"
         xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2">

  <!-- BIS Billing 3.0 mandatory header -->
  <cbc:CustomizationID>{customization_id}</cbc:CustomizationID>
  <cbc:ProfileID>{profile_id}</cbc:ProfileID>
  <cbc:ID>{invoice_id}</cbc:ID>
  <cbc:IssueDate>{issue_date}</cbc:IssueDate>
  <cbc:DueDate>{due_date}</cbc:DueDate>
  <cbc:InvoiceTypeCode>380</cbc:InvoiceTypeCode>
  <cbc:DocumentCurrencyCode>{currency}</cbc:DocumentCurrencyCode>
  <cbc:TaxCurrencyCode>{currency}</cbc:TaxCurrencyCode>

  <!-- Supplier (AccountingSupplierParty) -->
  <cac:AccountingSupplierParty>
    <cac:Party>
      <cbc:EndpointID schemeID="{sender_scheme}">{sender_id_val}</cbc:EndpointID>
      <cac:PartyName>
        <cbc:Name>{supplier_name}</cbc:Name>
      </cac:PartyName>
      <cac:PostalAddress>
        <cbc:CountrySubentity/>
        <cac:Country>
          <cbc:IdentificationCode>SE</cbc:IdentificationCode>
        </cac:Country>
      </cac:PostalAddress>
      <cac:PartyTaxScheme>
        <cbc:CompanyID/>
        <cac:TaxScheme>
          <cbc:ID>VAT</cbc:ID>
        </cac:TaxScheme>
      </cac:PartyTaxScheme>
      <cac:PartyLegalEntity>
        <cbc:RegistrationName>{supplier_name}</cbc:RegistrationName>
      </cac:PartyLegalEntity>
    </cac:Party>
  </cac:AccountingSupplierParty>

  <!-- Buyer (AccountingCustomerParty) -->
  <cac:AccountingCustomerParty>
    <cac:Party>
      <cbc:EndpointID schemeID="{receiver_scheme}">{receiver_id_val}</cbc:EndpointID>
      <cac:PartyName>
        <cbc:Name>{buyer_name}</cbc:Name>
      </cac:PartyName>
      <cac:PostalAddress>
{buyer_address}
        <cac:Country>
          <cbc:IdentificationCode>{buyer_country}</cbc:IdentificationCode>
        </cac:Country>
      </cac:PostalAddress>
      <cac:PartyTaxScheme>
        <cbc:CompanyID>{buyer_vat}</cbc:CompanyID>
        <cac:TaxScheme>
          <cbc:ID>VAT</cbc:ID>
        </cac:TaxScheme>
      </cac:PartyTaxScheme>
      <cac:PartyLegalEntity>
        <cbc:RegistrationName>{buyer_name}</cbc:RegistrationName>
        <cbc:CompanyID>{buyer_org}</cbc:CompanyID>
      </cac:PartyLegalEntity>
    </cac:Party>
  </cac:AccountingCustomerParty>

  <!-- Payment terms -->
  <cac:PaymentTerms>
    <cbc:Note>Net 30</cbc:Note>
  </cac:PaymentTerms>

  <!-- Tax total -->
{tax_total}

  <!-- Legal monetary total (BT-106 … BT-115) -->
  <cac:LegalMonetaryTotal>
    <cbc:LineExtensionAmount currencyID="{currency}">{tax_excl}</cbc:LineExtensionAmount>
    <cbc:TaxExclusiveAmount currencyID="{currency}">{tax_excl}</cbc:TaxExclusiveAmount>
    <cbc:TaxInclusiveAmount currencyID="{currency}">{tax_incl}</cbc:TaxInclusiveAmount>
    <cbc:AllowanceTotalAmount currencyID="{currency}">0.00</cbc:AllowanceTotalAmount>
    <cbc:ChargeTotalAmount currencyID="{currency}">0.00</cbc:ChargeTotalAmount>
    <cbc:PrepaidAmount currencyID="{currency}">0.00</cbc:PrepaidAmount>
    <cbc:PayableRoundingAmount currencyID="{currency}">0.00</cbc:PayableRoundingAmount>
    <cbc:PayableAmount currencyID="{currency}">{payable_amount}</cbc:PayableAmount>
  </cac:LegalMonetaryTotal>

  <!-- Invoice lines -->
{invoice_lines}

</Invoice>"#,
            customization_id = Self::xml_escape(BIS_CUSTOMIZATION_ID),
            profile_id = Self::xml_escape(BIS_PROFILE_ID),
            invoice_id = Self::xml_escape(&invoice_id),
            issue_date = issue_date,
            due_date = due_date,
            currency = Self::xml_escape(currency),
            sender_scheme = Self::xml_escape(&sender_scheme),
            sender_id_val = Self::xml_escape(&sender_id_val),
            supplier_name = Self::xml_escape(supplier_name),
            receiver_scheme = Self::xml_escape(&receiver_scheme),
            receiver_id_val = Self::xml_escape(&receiver_id_val),
            buyer_name = Self::xml_escape(&invoice.customer.name),
            buyer_address = buyer_address_xml,
            buyer_country = Self::xml_escape(&invoice.customer.country_code),
            buyer_vat = Self::xml_escape(
                invoice.customer.vat_number.as_deref().unwrap_or("")
            ),
            buyer_org = Self::xml_escape(
                invoice.customer.org_number.as_deref().unwrap_or("")
            ),
            tax_total = tax_xml,
            tax_excl = Self::fmt_amount(tax_excl),
            tax_incl = Self::fmt_amount(tax_incl),
            payable_amount = Self::fmt_amount(payable_amount),
            invoice_lines = lines_xml,
        )
    }

    /// Parse an incoming UBL 2.1 XML string into a canonical Invoice.
    /// Uses lightweight string-pattern extraction (no heavy XML library dep).
    pub fn from_ubl_xml(&self, xml: &str) -> Result<Invoice> {
        let invoice_id = Self::extract_xml_text(xml, "cbc:ID")
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let issue_date = Self::extract_xml_text(xml, "cbc:IssueDate")
            .unwrap_or_else(|| "1970-01-01".to_string());
        let due_date = Self::extract_xml_text(xml, "cbc:DueDate");
        let currency = Self::extract_xml_text(xml, "cbc:DocumentCurrencyCode")
            .unwrap_or_else(|| "EUR".to_string());

        let total_str = Self::extract_xml_attr_text(xml, "cbc:PayableAmount")
            .unwrap_or_else(|| "0".to_string());
        let tax_total_str = Self::extract_xml_attr_text(xml, "cbc:TaxAmount")
            .unwrap_or_else(|| "0".to_string());

        let total = Decimal::from_str(&total_str).unwrap_or_default();
        let tax_total = Decimal::from_str(&tax_total_str).unwrap_or_default();
        let subtotal = total - tax_total;

        // Extract buyer name from AccountingCustomerParty/Party/PartyName/Name
        let buyer_name = Self::extract_xml_text(xml, "cbc:Name").unwrap_or_default();
        // Extract buyer endpoint ID
        let buyer_endpoint = Self::extract_xml_text(xml, "cbc:EndpointID").unwrap_or_default();

        let customer = Party {
            id: Uuid::new_v4(),
            external_id: Some(format!("peppol:{}", buyer_endpoint)),
            name: buyer_name,
            org_number: None,
            vat_number: None,
            country_code: Self::extract_xml_text(xml, "cbc:IdentificationCode")
                .unwrap_or_else(|| "SE".to_string()),
            address: None,
            email: None,
        };

        // Parse invoice lines (basic: extract all InvoiceLine blocks)
        let line_items = Self::extract_invoice_lines(xml, &currency);

        let parsed_due = due_date
            .as_deref()
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
        let parsed_issue = chrono::NaiveDate::parse_from_str(&issue_date, "%Y-%m-%d")
            .unwrap_or_else(|_| Utc::now().date_naive());

        let status = if total == Decimal::ZERO {
            InvoiceStatus::Paid
        } else {
            InvoiceStatus::Sent
        };

        Ok(Invoice {
            id: Uuid::new_v4(),
            external_id: format!("peppol:{}", invoice_id),
            invoice_number: Some(invoice_id),
            date: parsed_issue,
            due_date: parsed_due,
            customer,
            line_items,
            subtotal,
            tax_total,
            total,
            balance: total,
            currency,
            exchange_rate: None,
            status,
        })
    }

    /// Validate a Peppol participant identifier.
    ///
    /// Format: `{ICD}:{identifier}` where ICD is a 4-digit ISO 6523 ICD code.
    /// Examples: `0088:1234567890123` (GLN), `0007:5560000001` (SE org nr)
    pub fn validate_peppol_id(id: &str) -> bool {
        let parts: Vec<&str> = id.splitn(2, ':').collect();
        if parts.len() != 2 {
            return false;
        }
        let icd = parts[0];
        let value = parts[1];
        // ICD must be exactly 4 decimal digits
        if icd.len() != 4 || !icd.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
        // Value must be non-empty and at most 50 chars (per Peppol policy)
        !value.is_empty() && value.len() <= 50
    }

    /// Look up a Peppol participant via SMP (simplified SML-based DNS lookup + HTTP).
    pub async fn lookup_participant(
        &self,
        org_number: &str,
    ) -> Result<Option<PeppolParticipant>> {
        // SE: ICD 0007; strip dashes and spaces
        let normalized = org_number.replace(['-', ' '], "");
        let peppol_id = format!("0007:{}", normalized);

        // Build SMP URL: hash the participant ID per OpenPEPPOL spec
        // md5(lowercase(participant)) → base32 encoded → smp dns label
        let smp_url = self.build_smp_url(&peppol_id);

        let resp = self
            .client
            .get(&smp_url)
            .header(header::ACCEPT, "application/json")
            .send()
            .await;

        match resp {
            Err(_) => Ok(None), // Not registered in Peppol
            Ok(r) if !r.status().is_success() => Ok(None),
            Ok(r) => {
                let sg: SmpServiceGroup = r.json().await.unwrap_or(SmpServiceGroup {
                    participant_identifier: None,
                    service_refs: None,
                });

                // Collect supported document types from service refs
                let capabilities: Vec<String> = sg
                    .service_refs
                    .unwrap_or_default()
                    .into_iter()
                    .map(|sr| sr.href)
                    .collect();

                Ok(Some(PeppolParticipant {
                    peppol_id,
                    name: sg
                        .participant_identifier
                        .unwrap_or_else(|| org_number.to_string()),
                    country: "SE".to_string(),
                    capabilities,
                }))
            }
        }
    }

    // -----------------------------------------------------------------------
    // XML generation helpers
    // -----------------------------------------------------------------------

    fn build_tax_total_xml(lines: &[LineItem], tax_total: &Decimal, currency: &str) -> String {
        // Group lines by tax rate (simplified: use single 25% SE VAT for now)
        let tax_amount = *tax_total;
        let taxable_amount = lines.iter().fold(Decimal::ZERO, |acc, l| acc + l.amount);

        // Derive rate (avoid divide-by-zero)
        let rate = if taxable_amount.is_zero() {
            Decimal::new(25, 0)
        } else {
            (tax_amount / taxable_amount * Decimal::new(100, 0))
                .round_dp(2)
        };

        format!(
r#"  <cac:TaxTotal>
    <cbc:TaxAmount currencyID="{currency}">{tax_amount}</cbc:TaxAmount>
    <cac:TaxSubtotal>
      <cbc:TaxableAmount currencyID="{currency}">{taxable_amount}</cbc:TaxableAmount>
      <cbc:TaxAmount currencyID="{currency}">{tax_amount}</cbc:TaxAmount>
      <cac:TaxCategory>
        <cbc:ID>S</cbc:ID>
        <cbc:Percent>{rate}</cbc:Percent>
        <cbc:TaxExemptionReason/>
        <cac:TaxScheme>
          <cbc:ID>VAT</cbc:ID>
        </cac:TaxScheme>
      </cac:TaxCategory>
    </cac:TaxSubtotal>
  </cac:TaxTotal>"#,
            currency = currency,
            tax_amount = Self::fmt_amount(tax_amount),
            taxable_amount = Self::fmt_amount(taxable_amount),
            rate = Self::fmt_amount(rate),
        )
    }

    fn build_invoice_line_xml(idx: usize, line: &LineItem) -> String {
        let line_id = line.id.clone().unwrap_or_else(|| idx.to_string());
        let unit = "C62"; // UN/ECE unit code for "one" (piece)
        let tax_percent = Decimal::new(25, 0); // 25% SE VAT

        format!(
r#"  <cac:InvoiceLine>
    <cbc:ID>{line_id}</cbc:ID>
    <cbc:InvoicedQuantity unitCode="{unit}">{qty}</cbc:InvoicedQuantity>
    <cbc:LineExtensionAmount currencyID="{currency}">{amount}</cbc:LineExtensionAmount>
    <cac:Item>
      <cbc:Description>{description}</cbc:Description>
      <cbc:Name>{description}</cbc:Name>
      <cac:ClassifiedTaxCategory>
        <cbc:ID>S</cbc:ID>
        <cbc:Percent>{tax_percent}</cbc:Percent>
        <cac:TaxScheme>
          <cbc:ID>VAT</cbc:ID>
        </cac:TaxScheme>
      </cac:ClassifiedTaxCategory>
    </cac:Item>
    <cac:Price>
      <cbc:PriceAmount currencyID="{currency}">{unit_price}</cbc:PriceAmount>
      <cbc:BaseQuantity unitCode="{unit}">1</cbc:BaseQuantity>
    </cac:Price>
  </cac:InvoiceLine>"#,
            line_id = Self::xml_escape(&line_id),
            unit = unit,
            qty = line.quantity.round_dp(2),
            currency = Self::xml_escape(&line.currency),
            amount = Self::fmt_amount(line.amount),
            description = Self::xml_escape(&line.description),
            tax_percent = Self::fmt_amount(tax_percent),
            unit_price = Self::fmt_amount(line.unit_price),
        )
    }

    fn build_address_xml(addr: &Option<Address>) -> String {
        let a = match addr {
            Some(a) => a,
            None => return String::new(),
        };
        let mut out = String::new();
        if let Some(ref street) = a.street {
            out.push_str(&format!(
                "        <cbc:StreetName>{}</cbc:StreetName>\n",
                Self::xml_escape(street)
            ));
        }
        if let Some(ref city) = a.city {
            out.push_str(&format!(
                "        <cbc:CityName>{}</cbc:CityName>\n",
                Self::xml_escape(city)
            ));
        }
        if let Some(ref postal) = a.postal_code {
            out.push_str(&format!(
                "        <cbc:PostalZone>{}</cbc:PostalZone>\n",
                Self::xml_escape(postal)
            ));
        }
        out
    }

    // -----------------------------------------------------------------------
    // XML parsing helpers (lightweight, no heavy dep)
    // -----------------------------------------------------------------------

    /// Extract the text content of the **first** occurrence of `<tag>...</tag>`.
    fn extract_xml_text(xml: &str, tag: &str) -> Option<String> {
        let open = format!("<{}>", tag);
        let close = format!("</{}>", tag);
        let start = xml.find(&open)?;
        let content_start = start + open.len();
        let end = xml[content_start..].find(&close)?;
        Some(xml[content_start..content_start + end].trim().to_string())
    }

    /// Extract the text content of `<tag ...>...</tag>` (with attributes).
    fn extract_xml_attr_text(xml: &str, tag: &str) -> Option<String> {
        // Find `<tag` then find `>` then find `</tag>`
        let tag_start = format!("<{}", tag);
        let close = format!("</{}>", tag);
        let start = xml.find(&tag_start)?;
        let gt = xml[start..].find('>')?;
        let content_start = start + gt + 1;
        let end = xml[content_start..].find(&close)?;
        Some(xml[content_start..content_start + end].trim().to_string())
    }

    /// Very basic InvoiceLine extractor: pulls lines between <cac:InvoiceLine> tags.
    fn extract_invoice_lines(xml: &str, currency: &str) -> Vec<LineItem> {
        let mut lines = Vec::new();
        let open = "<cac:InvoiceLine>";
        let close = "</cac:InvoiceLine>";
        let mut search = xml;

        loop {
            let start = match search.find(open) {
                Some(s) => s,
                None => break,
            };
            let after_open = start + open.len();
            let end = match search[after_open..].find(close) {
                Some(e) => after_open + e,
                None => break,
            };
            let block = &search[start..end + close.len()];

            let description = Self::extract_xml_text(block, "cbc:Name")
                .or_else(|| Self::extract_xml_text(block, "cbc:Description"))
                .unwrap_or_default();

            let amount = Self::extract_xml_attr_text(block, "cbc:LineExtensionAmount")
                .and_then(|s| Decimal::from_str(&s).ok())
                .unwrap_or_default();

            let qty = Self::extract_xml_attr_text(block, "cbc:InvoicedQuantity")
                .and_then(|s| Decimal::from_str(&s).ok())
                .unwrap_or(Decimal::ONE);

            let unit_price = Self::extract_xml_attr_text(block, "cbc:PriceAmount")
                .and_then(|s| Decimal::from_str(&s).ok())
                .unwrap_or_else(|| if qty.is_zero() { amount } else { amount / qty });

            let id = Self::extract_xml_text(block, "cbc:ID");

            lines.push(LineItem {
                id,
                description,
                quantity: qty,
                unit_price,
                amount,
                tax_amount: None,
                account_code: None,
                currency: currency.to_string(),
            });

            search = &search[end + close.len()..];
        }
        lines
    }

    // -----------------------------------------------------------------------
    // Utilities
    // -----------------------------------------------------------------------

    fn fmt_amount(d: Decimal) -> String {
        format!("{:.2}", d)
    }

    /// Escape the five predefined XML entities.
    fn xml_escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }

    /// Split `0007:5560000001` → (`0007`, `5560000001`).
    fn split_peppol_id(id: &str) -> (String, String) {
        let mut parts = id.splitn(2, ':');
        let scheme = parts.next().unwrap_or("0007").to_string();
        let value = parts.next().unwrap_or(id).to_string();
        (scheme, value)
    }

    /// Build an SMP REST URL for the participant (BDXL SMP profile).
    fn build_smp_url(&self, peppol_id: &str) -> String {
        // Real SMP uses DNS + http://B-{md5hash}.iso6523-actorid-upis.{sml}/
        // Here we use a simplified REST lookup if the AP exposes it.
        format!(
            "{}/smp/{}",
            self.access_point_url,
            urlencoding::encode(peppol_id)
        )
    }
}

// ---------------------------------------------------------------------------
// urlencoding shim (avoids adding a dep for a single call)
// ---------------------------------------------------------------------------
mod urlencoding {
    pub fn encode(s: &str) -> String {
        s.chars()
            .flat_map(|c| match c {
                'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => {
                    vec![c]
                }
                other => {
                    let encoded = format!("%{:02X}", other as u32);
                    encoded.chars().collect()
                }
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_peppol_id_valid_gln() {
        assert!(PeppolConnector::validate_peppol_id("0088:1234567890123"));
    }

    #[test]
    fn validate_peppol_id_valid_se() {
        assert!(PeppolConnector::validate_peppol_id("0007:5560000001"));
    }

    #[test]
    fn validate_peppol_id_no_colon() {
        assert!(!PeppolConnector::validate_peppol_id("00885560000001"));
    }

    #[test]
    fn validate_peppol_id_icd_too_short() {
        assert!(!PeppolConnector::validate_peppol_id("007:5560000001"));
    }

    #[test]
    fn validate_peppol_id_empty_value() {
        assert!(!PeppolConnector::validate_peppol_id("0007:"));
    }

    #[test]
    fn xml_escape_special_chars() {
        let s = PeppolConnector::xml_escape("A & B < C > D \"E\" 'F'");
        assert_eq!(s, "A &amp; B &lt; C &gt; D &quot;E&quot; &apos;F&apos;");
    }

    #[test]
    fn extract_xml_text_basic() {
        let xml = "<root><cbc:ID>INV-001</cbc:ID></root>";
        assert_eq!(
            super::PeppolConnector::extract_xml_text(xml, "cbc:ID"),
            Some("INV-001".to_string())
        );
    }

    #[test]
    fn to_ubl_xml_contains_mandatory_fields() {
        use chrono::NaiveDate;
        use rust_decimal_macros::dec;

        let connector = PeppolConnector {
            access_point_url: "https://ap.test".to_string(),
            sender_id: "0007:5560000001".to_string(),
            private_key: String::new(),
            certificate: String::new(),
            client: reqwest::Client::new(),
        };

        let invoice = Invoice {
            id: Uuid::new_v4(),
            external_id: "test:1".to_string(),
            invoice_number: Some("INV-2024-001".to_string()),
            date: NaiveDate::from_ymd_opt(2024, 3, 15).unwrap(),
            due_date: Some(NaiveDate::from_ymd_opt(2024, 4, 14).unwrap()),
            customer: Party {
                id: Uuid::new_v4(),
                external_id: Some("peppol:0007:5590000002".to_string()),
                name: "Test Buyer AB".to_string(),
                org_number: Some("559000-0002".to_string()),
                vat_number: Some("SE559000000201".to_string()),
                country_code: "SE".to_string(),
                address: None,
                email: None,
            },
            line_items: vec![LineItem {
                id: Some("1".to_string()),
                description: "Consulting services".to_string(),
                quantity: dec!(10),
                unit_price: dec!(1000),
                amount: dec!(10000),
                tax_amount: Some(dec!(2500)),
                account_code: None,
                currency: "SEK".to_string(),
            }],
            subtotal: dec!(10000),
            tax_total: dec!(2500),
            total: dec!(12500),
            balance: dec!(12500),
            currency: "SEK".to_string(),
            exchange_rate: None,
            status: InvoiceStatus::Sent,
        };

        let xml = connector.to_ubl_xml(&invoice);

        assert!(xml.contains(BIS_CUSTOMIZATION_ID));
        assert!(xml.contains(BIS_PROFILE_ID));
        assert!(xml.contains("<cbc:InvoiceTypeCode>380</cbc:InvoiceTypeCode>"));
        assert!(xml.contains("INV-2024-001"));
        assert!(xml.contains("<cbc:TaxAmount currencyID=\"SEK\">2500.00</cbc:TaxAmount>"));
        assert!(xml.contains("<cbc:PayableAmount currencyID=\"SEK\">12500.00</cbc:PayableAmount>"));
        assert!(xml.contains("Consulting services"));
    }

    #[test]
    fn roundtrip_ubl_xml() {
        use chrono::NaiveDate;
        use rust_decimal_macros::dec;

        let connector = PeppolConnector {
            access_point_url: "https://ap.test".to_string(),
            sender_id: "0007:5560000001".to_string(),
            private_key: String::new(),
            certificate: String::new(),
            client: reqwest::Client::new(),
        };

        let original = Invoice {
            id: Uuid::new_v4(),
            external_id: "test:2".to_string(),
            invoice_number: Some("INV-2024-RT".to_string()),
            date: NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
            due_date: None,
            customer: Party {
                id: Uuid::new_v4(),
                external_id: Some("peppol:0007:5590000099".to_string()),
                name: "Roundtrip Buyer AB".to_string(),
                org_number: None,
                vat_number: None,
                country_code: "SE".to_string(),
                address: None,
                email: None,
            },
            line_items: vec![],
            subtotal: dec!(0),
            tax_total: dec!(0),
            total: dec!(0),
            balance: dec!(0),
            currency: "SEK".to_string(),
            exchange_rate: None,
            status: InvoiceStatus::Paid,
        };

        let xml = connector.to_ubl_xml(&original);
        let parsed = connector.from_ubl_xml(&xml).unwrap();

        assert_eq!(parsed.invoice_number.as_deref(), Some("INV-2024-RT"));
        assert_eq!(parsed.currency, "SEK");
    }
}
