# Nästa steg – prioritetsordning

## 🔴 BLOCK (måste fixas innan production)

### 1. Fortnox credentials (Winston)
```
→ apps.fortnox.se/integrations/
→ Skapa app: Kvittovalvet
→ Scopes: Bokföring, Faktura, Leverantörsfaktura, Verifikation, Fil
→ Lägg client_id + client_secret i ~/.openclaw/secrets/fortnox.json
```
**Utan detta:** ingen ERP-data, matching engine körs på tom data.

### 2. Enable Banking → production
```
→ app.enablebanking.com
→ Byt från sandbox till production
→ Starta ny PSU-konsentsession (OAuth redirect)
→ Uppdatera ENABLE_BANKING_SESSION i .env
```
**Utan detta:** banktransaktioner är testdata, inte riktiga.

---

## 🟡 VIKTIGT (nästa sprint)

### 3. PostgreSQL-integration i API
Connectors hämtar live data men lagrar inget.
Behövs: sqlx-pool i AppState, spara transactions + match-resultat till DB.

### 4. Revolut production-verify
Verifiera om refresh_token är production eller sandbox.
```bash
curl -s https://b2b.revolut.com/api/1.0/accounts \
  -H "Authorization: Bearer <token>"
```

### 5. Fortnox connector implementation
src/connectors/fortnox.rs är stub.
Implementera fetch_transactions + fetch_invoices när credentials finns.

---

## 🟢 NICE TO HAVE (fas 2)

### 6. Webhook-mottagning live
- Revolut webhooks → POST /webhooks/revolut
- Enable Banking → callback URL

### 7. Peppol accesspunkt
Kontakta Inexchange (~500 kr/mån).

### 8. Frontend ↔ API live-koppling
Just nu visar frontend demo-data.
POST /sync/bank → GET /transactions flöde.

### 9. Email-retrieval (receipt recovery)
src/agents/receipt_recovery.rs är klar.
Behöver: IMAP-credentials, outbox-service.

---

## Vad som FUNGERAR just nu (2026-05-24)

| Komponent | Status |
|-----------|--------|
| Rust API /health | ✅ Live |
| PostgreSQL | ✅ Kör (Docker) |
| Redis | ✅ Kör (Docker) |
| NATS JetStream | ✅ Kör (Docker) |
| Matching Engine | ✅ Kompilerad |
| Enable Banking connector | ✅ Kompilerad (sandbox) |
| Revolut connector | ✅ Kompilerad |
| Nordea connector | ✅ Kompilerad (sandbox) |
| Next.js frontend | ✅ Live (localhost:3001) |
| Fortnox connector | ⚠️ Stub (ingen token) |
| DB-persistering | ⚠️ Ej kopplad till routes |
| Receipt recovery agent | ✅ Kompilerad (behöver IMAP) |
