# 🔄 OUROBOROS — Autonomt Revisionssystem

> *Det system som aldrig slutar granska sig självt*

**Företag:** LandveX AB (org.nr 559141-7042)  
**Produkt:** Wavult OS — `os.wavult.com`  
**API:** `api.wavult.com`  
**Status:** Under aktiv utveckling | 2026-05-23  
**Standard:** Utformad för att möta Skatteverkets framtida digitala revisionskrav

---

## 🗺️ VAR VI BEFINNER OSS (2026-05-23)

### Systemkarta
```
┌─────────────────────────────────────────────────────────┐
│                    WAVULT OS                            │
│              os.wavult.com (✅ Online)                  │
│                                                         │
│  ┌──────────────┐    ┌──────────────┐                  │
│  │  Frontend    │    │ Ouroboros    │                   │
│  │  React/Next  │    │ Demo (local) │                   │
│  └──────┬───────┘    └──────────────┘                  │
│         │                                               │
│  ┌──────▼───────────────────────────────────┐          │
│  │         api.wavult.com (⚠️ Degraded)     │          │
│  │  ┌─────────────────────────────────┐     │          │
│  │  │   Reconciler Core (Rust)        │     │          │
│  │  │   - Ledger Engine               │     │          │
│  │  │   - VAT Agent                   │     │          │
│  │  │   - Audit Trail                 │     │          │
│  │  │   - Entity Resolution           │     │          │
│  │  │   - OCR Pipeline                │     │          │
│  │  └─────────────────────────────────┘     │          │
│  │  ┌─────────────────────────────────┐     │          │
│  │  │   PostgreSQL (✅ OK)             │     │          │
│  │  │   NATS (events)                 │     │          │
│  │  └─────────────────────────────────┘     │          │
│  └──────────────────────────────────────────┘          │
│                                                         │
│  AMOS Server: 16.170.83.169 (eu-north-1 AWS)           │
└─────────────────────────────────────────────────────────┘
```

### Kända Issues (2026-05-23)
| Issue | Status | Ansvarig |
|-------|--------|----------|
| JWT-validering avvisar tokens | 🔴 AKUT | Johan (CTO) |
| SSH port 22 stängd på AMOS | 🔴 AKUT | Johan (CTO) |
| Nordea connector är stub | 🟡 | Dev |
| supabase_client i health-endpoint | 🟡 | Johan (CTO) |
| SRU/SIE4/SAF-T export saknas | 🟠 | Dev |

---

## 🏗️ ARKITEKTUR

### Tech Stack
| Layer | Teknologi | Status |
|-------|-----------|--------|
| Backend Core | Rust (Tokio async) | ✅ Implementerat |
| Web Framework | Axum | ✅ |
| Databas | PostgreSQL + SQLx | ✅ |
| Messaging | NATS JetStream | ✅ |
| Auth | Clerk (JWT) | ⚠️ Config-issue |
| AI/ML | OpenAI GPT-4o | ✅ |
| OCR | AWS Textract | ✅ |
| Frontend | React/Next.js | ✅ |
| Demo | HTML/CSS/JS (Ouroboros) | ✅ |
| Infra | AWS EC2 eu-north-1 | ✅ |
| Container | Docker + docker-compose | ✅ |

### Modulstruktur (Rust)
```
reconciler-core/
├── src/
│   ├── main.rs              # Entry point, Axum server
│   ├── api/                 # REST API handlers
│   ├── agents/              # Autonoma AI-agenter
│   │   ├── ap_automation    # Accounts Payable automation
│   │   ├── audit_agent      # Kontinuerlig revisionsgranskning
│   │   ├── receipt_recovery # Kvittoåterhämtning
│   │   └── vat_agent        # Momsautomatisering
│   ├── ai/                  # AI-integration
│   │   └── confidence       # Konfidenspoäng för AI-beslut
│   ├── auth/                # Autentisering (Clerk JWT)
│   ├── compliance/          # Regelefterlevnad
│   │   ├── audit            # Revisionsloggar
│   │   └── jurisdiction     # Jurisdiktionsspecifika regler (SE/EU)
│   ├── connectors/          # Bankintegrationer
│   │   ├── nordea           # Nordea Open Banking (STUB)
│   │   ├── revolut          # Revolut Business (AKTIV)
│   │   ├── fortnox          # Fortnox ERP
│   │   ├── tink             # Tink (aggregator)
│   │   ├── visma            # Visma
│   │   ├── xero             # Xero
│   │   ├── quickbooks       # QuickBooks
│   │   ├── kivra            # Kivra (digital post)
│   │   └── stripe_connector # Stripe
│   ├── db/                  # Databaslagret
│   │   └── repositories/    # Repository pattern
│   ├── entities/            # Entity Resolution
│   ├── events/              # NATS event-system
│   ├── graph/               # Financial Knowledge Graph
│   │   └── financial_graph  # Merchant relationship graph
│   ├── intelligence/        # AI-intelligens
│   │   ├── calibration      # Modellkalibrering
│   │   ├── entity_resolution # Automatisk enhetsidentifiering
│   │   └── merchant_graph   # Handelsman-relationer
│   ├── invoice_networks/    # Fakturanätverk
│   │   └── peppol           # Peppol BIS e-faktura
│   ├── ledger/              # Dubbel bokföring
│   │   ├── accounts         # BAS-kontoplan
│   │   └── journal          # Verifikationsjournal
│   ├── models/              # Datamodeller
│   ├── observability/       # Metrics + tracing
│   ├── ocr/                 # OCR-pipeline
│   │   ├── parser           # Dokumenttolkning
│   │   └── fraud_detection  # Bedrägeridetektion
│   ├── permissions/         # RBAC-behörigheter
│   ├── sandbox/             # Testmiljö
│   │   ├── bank_simulator   # Simulerad bankdata
│   │   ├── data_generator   # Testdatagenerering
│   │   ├── erp_simulator    # ERP-simulation
│   │   └── scenario_runner  # Scenariotester
│   └── treasury/            # Kassahantering
├── migrations/              # PostgreSQL-migrations (V001–V005)
├── docker-compose.yml       # Lokal dev-miljö
└── Cargo.toml               # Dependencies
```

---

## 🔌 API-ENDPOINTS

### Bas-URL: `https://api.wavult.com`

| Endpoint | Method | Auth | Status | Beskrivning |
|----------|--------|------|--------|-------------|
| /health | GET | Nej | ✅ | Systemhälsa |
| /v1/auth/login | POST | Nej | ✅ | Login → JWT |
| /v1/connections | GET | JWT | ⚠️ JWT-bug | Bankintegrationer |
| /v1/banks | GET | JWT | ⚠️ JWT-bug | Tillgängliga banker |
| /v1/accounts | GET | JWT | ⚠️ JWT-bug | Konton |
| /v1/integrations | GET | JWT | ⚠️ JWT-bug | Integrationsstatus |

### Health Response
```json
{
  "status": "degraded",
  "version": "1.0.0",
  "services": [
    {"name": "database", "status": "ok"},
    {"name": "supabase_client", "status": "fallback"}
  ]
}
```

### JWT Token (korrekt genererad, men avvisas):
```json
{
  "sub": "d43f5fd3-6df0-43a8-a4ea-17aa519ddab0",
  "email": "winston@hypbit.com",
  "org": "wavult-group",
  "roles": ["finance_admin", "group-cfo", "group-admin", "admin"],
  "iss": "identity.wavult.com",
  "aud": ["wavult-os", "quixzoom", "landvex"],
  "exp": 1779540287
}
```
**Problem:** `identity.wavult.com` signerar med en secret. `api.wavult.com` validerar med en ANNAN secret. Behöver synkas.

---

## 🏦 BANKINTEGRATIONER

| Bank/System | Typ | Status | Credentials |
|-------------|-----|--------|-------------|
| Nordea | Open Banking PSD2 | ⚠️ SANDBOX | `nordea-api-credentials.json` |
| Revolut Business | REST API | ✅ Konfigurerad | `revolut-business-api.json` |
| Tink | Aggregator | 🔧 Stub | - |
| Fortnox | ERP | 🔧 Stub | Saknas |
| Visma | ERP | 🔧 Stub | - |
| Peppol | E-faktura | 🔧 Stub | - |
| Kivra | Digital post | 🔧 Stub | - |

### Nordea Open Banking
- **Client ID:** `e2e761c5fc1f915c95722f4b1ed9fe42`
- **Redirect URI:** `https://wavult.com/nordea/callback`
- **API Base:** `https://api.nordeaopenbanking.com`
- **Environment:** SANDBOX → behöver byta till PRODUCTION
- **OAuth Flow:** Authorization Code (PSD2 SCA-compliant)

---

## 📊 DATABAS (PostgreSQL)

### Migrations (V001–V005):
- `V001__initial_schema.sql` — Grundschema (transaktioner, konton, verifikationer)
- `V002__audit_immutable.sql` — Immutable audit trail (INSERT only, no UPDATE/DELETE)
- `V003__indexes.sql` — Performance-index
- `V004__bas_accounts.sql` — BAS 2024 kontoplan (hela svenska kontoplanen)
- `V005__test_data.sql` — Testdata för sandbox

### Nyckelentiteter:
- `transactions` — Alla finansiella transaktioner
- `journal_entries` — Dubbelbokföringsposter
- `audit_log` — Immutable revisionslogg
- `bas_accounts` — BAS 2024 kontoplan
- `entities` — Företag/motparter (entity resolution)

---

## 🤖 AI-AGENTER

| Agent | Uppgift | Status |
|-------|---------|--------|
| `ap_automation` | Automatiserar leverantörsbetalningar | ✅ Implementerad |
| `audit_agent` | Kontinuerlig revisionsgranskning | ✅ Implementerad |
| `receipt_recovery` | Hittar saknade kvitton | ✅ Implementerad |
| `vat_agent` | Automatisk momshantering | ✅ Implementerad |

### AI-pipeline:
```
Dokument → OCR (AWS Textract) → Parser → Entity Resolution
    → Confidence Score → Ledger Entry → Audit Check → Arkiv
```

---

## 🏛️ SKATTEVERKET-COMPLIANCE (REVISIONSRAV)

### Implementerat ✅
- Dubbel bokföring (debet = kredit)
- Immutable audit trail
- BAS 2024 kontoplan
- Verifikationsnumrering i löpande följd
- Data lineage tracking

### Saknas ❌
| Standard | Format | Prioritet |
|----------|--------|-----------|
| Momsdeklaration | SRU | 🔴 Hög |
| Bokföringsdata | SIE4 | 🔴 Hög |
| Skatterevision | SAF-T | 🟡 Medium |
| E-faktura | Peppol BIS 3.0 | 🟡 Medium |
| Inkomstdeklaration | INK2R XML | 🟠 |
| Arbetsgivardeklaration | AGI XML | 🟠 |

---

## 🎮 OUROBOROS DEMO

Live demo: `http://localhost:9977/ouroboros-demo.html`

### Vyer (12 st):
1. **Ouroboros** — Animerad ouroboros + live KPI-räknare
2. **Live Audit** — Realtidsaudit-pipeline med inject-scenarios
3. **Revisionsrav** — 15 RAV-krav med status + localStorage
4. **Skatteverket** — SRU/INK2R/AGI med nedladdningsbara filer
5. **Integrationer** — 8 integrationer med status
6. **Ledger** — BAS 2024, T-konton, dubbelbokföring
7. **Compliance** — Score-gauge + GDPR/PSD2/Bokföringslagen
8. **Händelseström** — Live event-feed med filter
9. **System** — Hälsostatus alla tjänster
10. **INK2R** — Inkomstdeklaration-formulär med XML-export
11. **AGI** — Arbetsgivardeklaration med XML-export
12. **Digital signering** — BankID-simulering + SHA-256 hash-kedja

### Tekniskt:
- Self-contained HTML (114 KB, 2823 rader)
- Inga externa dependencies
- localStorage persistence
- Riktiga nedladdningsbara filer (SRU, SIE4, SAF-T, INK2R XML, AGI XML)

---

## 🖥️ INFRASTRUKTUR

### Servrar (AWS eu-north-1)
| Server | IP | Tjänst | Status |
|--------|-----|--------|--------|
| AMOS | 16.170.83.169 | os.wavult.com + api | ✅ Online |
| API Load Balancer | 13.61.47.182 | api.wavult.com | ✅ Online |
| API Load Balancer | 13.62.132.177 | api.wavult.com | ✅ Online |

### Lokalt (WSL2 / DESKTOP-4PSTVB7)
| Tjänst | Port | Status |
|--------|------|--------|
| Rufus (OpenClaw) | 18789 | ✅ Running |
| Ouroboros Demo | 9977 | ✅ Running |
| Chromium (browser) | 18800 | ✅ Running |

### AI-infrastruktur
| System | Roll | Status |
|--------|------|--------|
| Rufus (OpenClaw lokal) | Winston's personliga AI | ✅ |
| Bernt (OpenClaw AWS) | Företagets AI | ✅ |

---

## 👥 TEAM

| Person | Roll | Kontakt |
|--------|------|---------|
| Winston Bjarnemark | CFO / Grundare | winston@hypbit.com |
| Johan (Sven) | CTO | @sven_135bot (Telegram) |
| Dennis | Dev | - |
| Leon | Dev | - |

---

## 🚀 NÄSTA STEG (Prioritetsordning)

1. **Johan fixar JWT-secret på AMOS** → alla API-endpoints fungerar
2. **SSH-access till AMOS** → kan debugga produktionsservern
3. **Nordea production-credentials** → riktig bankdata
4. **SRU-export implementation** → Skatteverket momsdeklaration
5. **SIE4-export** → extern revision möjlig
6. **PostgreSQL som default** (verifiera)
7. **Peppol e-faktura** → EU-standard
8. **SAF-T export** → OECD-revisionsstandard

---

## 📁 REPO-STRUKTUR

```
ouroboros/
├── README.md                    # Denna fil
├── reconciler-core/             # Rust backend (hela kodbasen)
│   ├── Cargo.toml
│   ├── Dockerfile
│   ├── docker-compose.yml
│   ├── .env.example
│   ├── migrations/              # PostgreSQL migrations (V001-V005)
│   ├── nats/                    # NATS-konfiguration
│   ├── scripts/                 # Dev-scripts
│   └── src/                     # All Rust-källkod
│       ├── main.rs
│       ├── agents/              # AP, Audit, Receipt, VAT
│       ├── ai/                  # Confidence scoring
│       ├── api/                 # REST handlers
│       ├── auth/                # Clerk JWT
│       ├── compliance/          # Audit + Jurisdiction
│       ├── connectors/          # Nordea, Revolut, Fortnox, etc.
│       ├── db/                  # PostgreSQL + repositories
│       ├── entities/            # Entity resolution
│       ├── events/              # NATS events
│       ├── graph/               # Financial knowledge graph
│       ├── intelligence/        # AI + merchant graph
│       ├── invoice_networks/    # Peppol
│       ├── ledger/              # Double-entry bookkeeping
│       ├── models/              # Data models
│       ├── observability/       # Metrics + tracing
│       ├── ocr/                 # OCR + fraud detection
│       ├── permissions/         # RBAC
│       ├── sandbox/             # Bank/ERP simulators
│       └── treasury/            # Cash management
├── demo/
│   └── ouroboros-demo.html      # Interactive compliance demo (114KB)
└── docs/
    └── REVISIONSRAV-WAVULT-OS-v1.md  # Audit requirements document
```

---

*Genererat av Rufus (OpenClaw) — 2026-05-23*  
*Ouroboros = systemet som aldrig slutar granska sig självt 🔄*
