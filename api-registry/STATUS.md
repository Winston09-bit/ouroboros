# Kvittovalvet – API Registry

> Enda källan till sanning för alla externa API-integrationer.
> Uppdatera denna fil när status ändras.

---

## 📊 Snabböversikt

### Banker / Bank-aggregatorer

| API | Typ | Status | Miljö | Connector | Prio |
|-----|-----|--------|-------|-----------|------|
| **Tink** | Aggregator (Visa) | ✅ Credentials klara | SANDBOX | `tink_impl.rs` | P0 |
| **Enable Banking** | Aggregator | ✅ Credentials klara | SANDBOX | `enable_banking.rs` | P0 |
| **Aiia** | Aggregator (Mastercard) | ⏳ Backup-option | – | – | P2 |
| **Revolut Business** | Bank (direkt) | ✅ Credentials klara | PROD | `revolut_impl.rs` | P0 |
| **Swedbank** | Storbank | 🔄 Via Tink/Enable | sandbox | aggregator | P0 |
| **Handelsbanken** | Storbank | 🔄 Via Tink/Enable | sandbox | aggregator | P0 |
| **SEB** | Storbank | 🔄 Via Tink/Enable | sandbox | aggregator | P0 |
| **Nordea** | Storbank | ✅ Direkt + via agg | SANDBOX | `nordea.rs` | P0 |
| **Wise** | Digital | ⏳ Eget API tillgängligt | – | – | P1 |
| **Lunar** | Digital | 🔄 Via Tink | – | aggregator | P2 |
| **Danske Bank** | Storbank | 🔄 Via Tink/Enable | sandbox | aggregator | P1 |
| **DNB** | Storbank (NO) | 🔄 Via Tink/Enable | sandbox | aggregator | P2 |

### ERP / Bokföring

| API | Typ | Status | Miljö | Prio |
|-----|-----|--------|-------|------|
| **Fortnox** | ERP (SE marknadsledare) | ⚠️ Email-bekräftelse expired – behöver ny ansökan | – | P0 |
| **Visma eEkonomi** | ERP | ❌ Saknar credentials – ansök på developer.visma.com | – | P0 |
| **Björn Lundén** | ERP | ❌ Inget öppet API – partner-only | – | P2 |
| **PE Accounting** | ERP | ❌ Saknar credentials | – | P1 |

### Merchants – Dagligvaror

| Merchant | Kategori | Receipt API | Email-mönster | Status |
|----------|----------|-------------|---------------|--------|
| ICA Gruppen | DAGLIGVAROR | ❌ Kund-API stängt | receipt@ica.se | Email-retrieval |
| Coop Sverige | DAGLIGVAROR | ❌ Endast app | medlem@coop.se | Email-retrieval |
| Willys | DAGLIGVAROR | ❌ Via Axfood | no-reply@willys.se | Email-retrieval |
| Hemköp | DAGLIGVAROR | ❌ Via Axfood | – | Email-retrieval |
| Lidl | DAGLIGVAROR | ❌ Endast app (Lidl Plus) | – | App-scraping |
| City Gross | DAGLIGVAROR | ❌ – | – | Manuell |
| Axfood | DAGLIGVAROR | ⚠️ Endast B2B | – | Inv-faktura |

### Merchants – Drivmedel

| Merchant | Kategori | Receipt API | Status |
|----------|----------|-------------|--------|
| Circle K | DRIVMEDEL | ⚠️ Extraclub app | App-only |
| OKQ8 | DRIVMEDEL | ⚠️ OKQ8 Kort | App-only |
| ST1 | DRIVMEDEL | ⚠️ ST1 Kort | App-only |
| Preem | DRIVMEDEL | ❌ – | Email |
| Tesla Supercharging | DRIVMEDEL | ✅ Tesla account | Web-portal |
| ChargeNode | DRIVMEDEL | ✅ Email | Auto-email |

### Merchants – Bygg/Elektronik

| Merchant | Kategori | Status |
|----------|----------|--------|
| Clas Ohlson | BYGG_ELEKTRONIK | Clas Ohlson Club app |
| Bauhaus | BYGG_ELEKTRONIK | Email |
| Biltema | BYGG_ELEKTRONIK | Biltema-konto |
| Jula | BYGG_ELEKTRONIK | Jula Club |
| Elgiganten | BYGG_ELEKTRONIK | Mina sidor |
| Power | BYGG_ELEKTRONIK | Email |
| NetOnNet | BYGG_ELEKTRONIK | Mina sidor |
| Webhallen | BYGG_ELEKTRONIK | Mina sidor |

### Hotell / Resor

| Merchant | Kategori | Status |
|----------|----------|--------|
| Scandic Hotels | HOTELL_RESOR | Scandic Friends |
| Strawberry (Nordic Choice) | HOTELL_RESOR | Email |
| SJ | HOTELL_RESOR | SJ-konto API |
| SAS | HOTELL_RESOR | EuroBonus |
| Booking.com | HOTELL_RESOR | Email |

### Revisionsbyråer (export-mål)

| Byrå | Format | Status |
|------|--------|--------|
| PwC | SIE/audit package | Framtida |
| KPMG | SIE/audit package | Framtida |
| EY | SIE/audit package | Framtida |
| Deloitte | SIE/audit package | Framtida |
| BDO | SIE/audit package | Framtida |
| Grant Thornton | SIE/audit package | Framtida |
| Mazars | SIE/audit package | Framtida |

### E-faktura / Standarder

| Standard | Användning | Status |
|----------|------------|--------|
| Peppol BIS 3.0 | E-fakturor B2B | ❌ Behöver AP |
| Svefaktura | Svenska e-fakturor | ❌ Via Peppol-AP |
| SIE 4 | Bokföringsexport | ✅ Stöd planerat |

### Infrastruktur / Övrigt

| Tjänst | Användning | Status |
|--------|------------|--------|
| AWS S3 | Filer | ⚠️ Ej konfigurerad |
| AWS SES | Email-skickande | ⚠️ Sandbox |
| AWS KMS | Kryptering | ⚠️ Ej konfigurerad |
| Resend | Transactional mail (alt.) | ⚠️ Ej konfigurerad |
| Twilio | SMS/röst | ⚠️ Ej konfigurerad |
| Stripe | Betalningar | ⚠️ Stub |
| OpenAI / Anthropic | AI-klassificering | ⚠️ Via wavult-keys |

---

## 🔐 Credentials lokalt

Alla i `~/.openclaw/secrets/`:

| Fil | Status |
|-----|--------|
| `tink.json` | ✅ NYTT – client_id + secret |
| `enable-banking.json` | ✅ app_id + session-UUID |
| `enable-banking-key.pem` | ✅ RSA private key |
| `revolut-business-api.json` | ✅ client_id + refresh_token |
| `revolut-private.pem` | ✅ RSA private key |
| `nordea-api-credentials.json` | ✅ sandbox client_id |
| `gmail-winston-wavult.env` | ✅ för OAuth retrieval |
| `email-hypbit.env` | ✅ IMAP för inbox-scanning |

---

## 🎯 ANSÖKNINGAR SOM BEHÖVER GÖRAS (Winston)

> Endast lista – inga ansökningar görs automatiskt.

1. **Fortnox developer-konto** – apps.fortnox.se/integrations (förra länken expired)
2. **Visma developer** – developer.visma.com
3. **Tink Production** – b2bmarketing.tink.com/contact-sales (kontakta sales)
4. **Enable Banking Production** – app.enablebanking.com
5. **Nordea Production** – developer.nordeaopenbanking.com
6. **PE Accounting API** – kontakta PE Accounting säljteam
7. **AWS-konto för Kvittovalvet** – S3, SES, KMS production
8. **Peppol-accesspunkt** – Inexchange eller Pagero (~500 kr/mån)
9. **Domäner** – kvittovalvet.se (kvittovalvet.com om .se taget)

---

## 📁 Mapp-struktur

```
api-registry/
├── STATUS.md              ← Denna fil
├── NEXT-STEPS.md          ← Prioriterad todo
├── CATEGORIES.md          ← Kategori-spec
├── banks/
│   ├── tink.md
│   ├── enable-banking.md
│   ├── revolut.md
│   ├── nordea.md
│   ├── swedbank.md
│   ├── handelsbanken.md
│   ├── seb.md
│   └── ...
├── erp/
│   ├── fortnox.md
│   ├── visma.md
│   └── pe-accounting.md
├── merchants/
│   ├── dagligvaror.md
│   ├── drivmedel.md
│   ├── bygg-elektronik.md
│   ├── hotell-resor.md
│   ├── restaurang.md
│   └── transport.md
├── e-faktura/
│   └── peppol.md
└── revisionsbyraer/
    └── README.md
```
