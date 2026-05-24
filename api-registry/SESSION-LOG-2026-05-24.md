# Session 2026-05-24 – Vad som byggts

## ✅ Klart i dag

### Banker
- **Revolut Business**: OAuth verifierat fungerande. Refresh-token-flöde returnerar access_token.
  - ⚠️ 403 IP-whitelist – Winston behöver lägga till IP på business.revolut.com
- **Tink**: Credentials klara (`tink.json`). Sandbox, 107 BIS-banker tillgängliga (Swedbank, Handelsbanken, SEB, Nordea + 100+ till).
- **Enable Banking**: JWT fungerar. Session kan vara stale.
- **Nordea**: Direkt-implementation + sandbox.

### Merchant Database (NYTT)
- 107 svenska merchants med full data
- 10 kategorier
- Bank-aliasing (5-10 alias per merchant)
- Receipt-retrieval-kanaler dokumenterade
- MerchantResolver med 5-stegs confidence-scoring

### Kivra-clone (Receipt Retrieval)
- Trait + Registry för retrieval-providers
- 5 första providers: ICA, OKQ8, Circle K, Clas Ohlson, Scandic
- Stubs som loggar exakta endpoints de skulle anropa

### Email Inbox
- IMAP-anslutning från env-fil (email-hypbit.env)
- Receipt-parser med regex för belopp/VAT/datum

### Matching Engine (förra sessionen)
- 5 scoring signals + Levenshtein
- Confidence thresholds: 0.7/0.4

### API
- 11 routes live på localhost:8090
- /sync/bank, /sync/status, /match, /evidence, /escalations

## 🔐 Credentials sparade
```
~/.openclaw/secrets/
├── tink.json                  ← NYTT
├── enable-banking.json
├── revolut-business-api.json
├── nordea-api-credentials.json
└── ...
```

## 📋 Lista över ansökningar Winston behöver göra
1. **Revolut IP-whitelist**: business.revolut.com → Settings → APIs → IP whitelist
2. **Fortnox developer**: Ny ansökan på apps.fortnox.se/integrations (förra länken expired)
3. **Visma developer**: developer.visma.com
4. **Tink production**: kontakta sales (b2bmarketing.tink.com/contact-sales)
5. **Enable Banking production**: app.enablebanking.com
6. **Nordea production**: developer.nordeaopenbanking.com
7. **Peppol-accesspunkt**: Inexchange/Pagero (~500 kr/mån)
8. **AWS-konto**: S3, SES, KMS

## 🚧 Inte byggt än men förberett
- Fortnox connector (väntar credentials)
- Visma connector (väntar credentials)
- Peppol AP-integration (väntar AP-avtal)
- Apotek, vård (kategori finns men inga providers)
- Revisionsbyråer (export-mål – senare)
- AWS-integrationer (väntar AWS-konto)

## 🔄 Server-status
- ✅ Postgres på 5433
- ✅ Redis på 6379
- ✅ NATS på 4222
- ✅ Reconciler API på 8090
- ✅ Next.js UI på 3001
- ✅ Allt kompilerar med 0 errors
