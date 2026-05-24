# Kvittovalvet – API Registry

> Enda källan till sanning för alla externa API-integrationer.
> Uppdatera denna fil när status ändras.

---

## Snabböversikt

| API | Typ | Status | Miljö | Prioritet |
|-----|-----|--------|-------|-----------|
| **Enable Banking** | Bank-aggregator | ✅ Konfigurerad | SANDBOX | P0 |
| **Nordea Open Banking** | Bank (direkt) | ✅ Konfigurerad | SANDBOX | P0 |
| **Revolut Business** | Bank | ✅ Konfigurerad | PRODUCTION? | P0 |
| **Fortnox** | ERP | ❌ Saknar credentials | – | P0 |
| **Visma eEkonomi** | ERP | ❌ Saknar credentials | – | P1 |
| **Peppol** | E-faktura | ❌ Behöver accesspunkt | – | P1 |
| **Tink** | Bank-aggregator (alt) | ❌ Saknar credentials | – | P2 |
| **Kivra** | Digital post | ❌ Saknar credentials | – | P2 |
| **Skatteverket** | Myndighet | ❌ Ingen publik API | – | P3 |
| **Stripe** | Betalningar | ❌ Saknar credentials | – | P2 |

---

## Steg-för-steg för att bli production-ready

### FAS 1 – Bank-data live (vecka 1-2)
- [x] Enable Banking sandbox uppsatt
- [x] Nordea sandbox uppsatt  
- [ ] Enable Banking → production (ansökan via enablebanking.com)
- [ ] Nordea → production (ansök på developer.nordeaopenbanking.com)
- [ ] Revolut → verifiera om production eller sandbox

### FAS 2 – ERP-integration (vecka 2-3)
- [ ] **Fortnox**: Skapa app på apps.fortnox.se → få `client_id` + `client_secret`
  - URL: https://apps.fortnox.se/integrations/
  - Behöver: Bokföring, Faktura, Leverantörsfaktura, Filarkiv
- [ ] **Visma**: Partner-program på developer.visma.com
  - URL: https://developer.visma.com/api/visma-administration/

### FAS 3 – E-faktura (vecka 3-4)
- [ ] **Peppol accesspunkt**: Netnordic, Inexchange, eller Pagero
  - Kostnad: ~500-1500 kr/mån
  - Kräver: Org.nr 559141-7042 (LandveX AB)

### FAS 4 – Enrichment (vecka 4+)
- [ ] Tink (via Visa): developer.tink.com
- [ ] Kivra: kivra.com/developer
- [ ] Stripe: dashboard.stripe.com

---

## Credentials-läge (uppdaterat 2026-05-24)

Se `~/.openclaw/secrets/` för faktiska värden.

| Fil | Innehåll |
|-----|----------|
| `enable-banking.json` | app_id, session-id, konto-UUIDs |
| `enable-banking-key.pem` | Private key för JWT-signering |
| `enablebanking-config.json` | app_id sandbox, private_key_path |
| `enablebanking-private.pem` | Private key (sandbox) |
| `nordea-api-credentials.json` | client_id, client_secret, sandbox |
| `revolut-business-api.json` | client_id, refresh_token, private_key |
| `revolut-private.pem` | Revolut JWT private key |

---

## Vad varje API ger oss

### Enable Banking (PRIORITET P0)
- Aggregerar 2000+ banker i Europa via ett API
- Vi har redan konto-UUIDs: sek_1, eur, sek_2, sek_3
- Ger: transaktioner, saldon, kontoinformation
- **Behövs för**: Bank-sync utan att bygga per-bank

### Nordea Open Banking (P0)
- Direkt Nordea-integration via PSD2
- Ger: transaktioner, saldon, betalningar
- Sandbox-miljö → behöver production-ansökan

### Revolut Business (P0)  
- Revolut Business API med JWT-autentisering
- Verifierat: client_id + refresh_token + RSA private key finns
- Ger: transaktioner, saldon, webhooks
- **Oklart**: production eller sandbox?

### Fortnox (P0 – SAKNAS)
- Viktigaste ERP-integrationen för Sverige
- ~40% av svenska SMEs använder Fortnox
- Behöver: client_id + client_secret från apps.fortnox.se
- **ÅTGÄRD**: Winston registrerar Fortnox-app

### Peppol (P1 – SAKNAS)
- Europeisk e-fakturastandard
- Krävs för B2B-fakturor till myndigheter
- Behöver: accesspunkt-avtal med en Peppol-AP-leverantör
- **ÅTGÄRD**: Kontakta Inexchange eller Pagero
