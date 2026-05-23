use async_trait::async_trait;
use anyhow::{Result, Context};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{AccountingProvider, ProviderHealth, HealthStatus};
use crate::models::*;

pub struct FortnoxConnector {
    client: Client,
    access_token: String,
    base_url: String,
}

impl FortnoxConnector {
    pub fn new(access_token: String) -> Self {
        Self {
            client: Client::new(),
            access_token,
            base_url: "https://api.fortnox.se/3".to_string(),
        }
    }

    fn auth_headers(&self) -> [(&str, &str); 2] {
        [
            ("Access-Token", &self.access_token),
            ("Content-Type", "application/json"),
        ]
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self.client.get(&url)
            .header("Access-Token", &self.access_token)
            .header("Content-Type", "application/json")
            .send().await
            .context("Fortnox HTTP request failed")?;
        resp.json::<T>().await.context("Fortnox JSON parse failed")
    }
}

#[derive(Deserialize)]
struct FortnoxVoucherResponse {
    #[serde(rename = "Voucher")]
    voucher: FortnoxVoucher,
}

#[derive(Deserialize)]
struct FortnoxVoucher {
    #[serde(rename = "VoucherNumber")]
    voucher_number: Option<String>,
}

#[derive(Serialize)]
struct FortnoxVoucherRequest {
    #[serde(rename = "Voucher")]
    voucher: FortnoxVoucherPayload,
}

#[derive(Serialize)]
struct FortnoxVoucherPayload {
    #[serde(rename = "Description")]
    description: String,
    #[serde(rename = "TransactionDate")]
    transaction_date: String,
    #[serde(rename = "VoucherRows")]
    rows: Vec<FortnoxVoucherRow>,
}

#[derive(Serialize)]
struct FortnoxVoucherRow {
    #[serde(rename = "Account")]
    account: String,
    #[serde(rename = "Debit")]
    debit: f64,
    #[serde(rename = "Credit")]
    credit: f64,
    #[serde(rename = "Description")]
    description: String,
}

#[async_trait]
impl AccountingProvider for FortnoxConnector {
    fn provider_id(&self) -> &str { "fortnox" }
    fn display_name(&self) -> &str { "Fortnox" }
    fn supported_jurisdictions(&self) -> Vec<String> { vec!["SE".to_string()] }

    async fn fetch_transactions(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> Result<Vec<Transaction>> {
        let path = format!("/vouchers?fromdate={}&todate={}", 
            from.format("%Y-%m-%d"), to.format("%Y-%m-%d"));
        
        let resp: serde_json::Value = self.get(&path).await?;
        let vouchers = resp["Vouchers"].as_array().cloned().unwrap_or_default();
        
        let txns = vouchers.iter().map(|v| Transaction {
            id: Uuid::new_v4(),
            external_id: v["VoucherNumber"].as_str().map(|s| s.to_string()),
            amount: v["VoucherRows"][0]["Debit"].as_f64()
                .map(|f| rust_decimal::Decimal::try_from(f).unwrap_or_default())
                .unwrap_or_default(),
            currency: "SEK".to_string(),
            timestamp: Utc::now(),
            counterparty: None,
            merchant: None,
            invoice_id: None,
            payment_rail: PaymentRail::SepaTransfer,
            jurisdiction: "SE".to_string(),
            tax_amount: None,
            tax_rate: None,
            account_id: v["VoucherSeries"].as_str().map(|s| s.to_string()),
            source: IntegrationSource::Fortnox,
            status: TransactionStatus::Booked,
            confidence: 1.0,
            audit_trail: vec![
                AuditEvent::new("system", "imported", "Fetched from Fortnox API", 1.0, "fortnox")
            ],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }).collect();
        
        Ok(txns)
    }

    async fn fetch_invoices(&self, _status: InvoiceStatus) -> Result<Vec<Invoice>> {
        let resp: serde_json::Value = self.get("/invoices").await?;
        let items = resp["Invoices"].as_array().cloned().unwrap_or_default();
        
        let invoices = items.iter().map(|inv| Invoice {
            id: Uuid::new_v4(),
            external_id: inv["DocumentNumber"].as_str().map(|s| s.to_string()),
            invoice_number: inv["DocumentNumber"].as_str().unwrap_or("").to_string(),
            vendor: Some(Party {
                id: None,
                name: inv["CustomerName"].as_str().unwrap_or("").to_string(),
                normalized_name: None,
                registration_number: None,
                vat_number: inv["VATNumber"].as_str().map(|s| s.to_string()),
                country: inv["Country"].as_str().map(|s| s.to_string()),
                entity_confidence: 0.9,
            }),
            customer: None,
            amount: rust_decimal::Decimal::try_from(inv["Total"].as_f64().unwrap_or(0.0)).unwrap_or_default(),
            tax_amount: rust_decimal::Decimal::try_from(inv["VAT"].as_f64().unwrap_or(0.0)).unwrap_or_default(),
            currency: inv["Currency"].as_str().unwrap_or("SEK").to_string(),
            issued_at: Utc::now(),
            due_at: None,
            status: InvoiceStatus::Received,
            source: IntegrationSource::Fortnox,
            jurisdiction: "SE".to_string(),
            line_items: vec![],
            documents: vec![],
            confidence: 1.0,
            audit_trail: vec![
                AuditEvent::new("system", "imported", "Fetched from Fortnox", 1.0, "fortnox")
            ],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }).collect();
        
        Ok(invoices)
    }

    async fn create_voucher(&self, voucher: &Voucher) -> Result<String> {
        let rows: Vec<FortnoxVoucherRow> = voucher.entries.iter().map(|e| FortnoxVoucherRow {
            account: e.account_code.clone(),
            debit: e.debit.try_into().unwrap_or(0.0),
            credit: e.credit.try_into().unwrap_or(0.0),
            description: e.description.clone(),
        }).collect();

        let payload = FortnoxVoucherRequest {
            voucher: FortnoxVoucherPayload {
                description: voucher.description.clone(),
                transaction_date: voucher.date.format("%Y-%m-%d").to_string(),
                rows,
            }
        };

        let resp = self.client.post(format!("{}/vouchers", self.base_url))
            .header("Access-Token", &self.access_token)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send().await?
            .json::<FortnoxVoucherResponse>().await?;

        Ok(resp.voucher.voucher_number.unwrap_or_default())
    }

    async fn sync_chart_of_accounts(&self) -> Result<Vec<Account>> {
        let resp: serde_json::Value = self.get("/accounts").await?;
        let items = resp["Accounts"].as_array().cloned().unwrap_or_default();
        
        Ok(items.iter().map(|a| Account {
            code: a["Number"].as_str().unwrap_or("").to_string(),
            name: a["Description"].as_str().unwrap_or("").to_string(),
            account_type: AccountType::Expense,
            currency: "SEK".to_string(),
            parent_code: None,
            jurisdiction: "SE".to_string(),
            is_active: a["Active"].as_bool().unwrap_or(true),
        }).collect())
    }

    async fn sync_vendors(&self) -> Result<Vec<Vendor>> {
        let resp: serde_json::Value = self.get("/suppliers").await?;
        let items = resp["Suppliers"].as_array().cloned().unwrap_or_default();
        
        Ok(items.iter().map(|s| Vendor {
            id: Uuid::new_v4(),
            party: Party {
                id: None,
                name: s["Name"].as_str().unwrap_or("").to_string(),
                normalized_name: None,
                registration_number: s["OrganisationNumber"].as_str().map(|s| s.to_string()),
                vat_number: s["VATNumber"].as_str().map(|s| s.to_string()),
                country: s["Country"].as_str().map(|s| s.to_string()),
                entity_confidence: 0.95,
            },
            default_account_code: s["Account"].as_str().map(|s| s.to_string()),
            payment_terms_days: s["TermsOfPayment"].as_str()
                .and_then(|s| s.parse().ok()),
            preferred_currency: s["Currency"].as_str().map(|s| s.to_string()),
            audit_trail: vec![],
            created_at: Utc::now(),
        }).collect())
    }

    async fn push_payment(&self, payment: &Payment) -> Result<String> {
        Ok(format!("FX-{}", payment.id))
    }

    async fn health_check(&self) -> Result<ProviderHealth> {
        let start = std::time::Instant::now();
        let ok = self.get::<serde_json::Value>("/companyinformation").await.is_ok();
        Ok(ProviderHealth {
            provider_id: "fortnox".to_string(),
            status: if ok { HealthStatus::Healthy } else { HealthStatus::Down },
            latency_ms: start.elapsed().as_millis() as u64,
            message: None,
        })
    }
}
