# Enable Banking

> PSD2-aggregator för 2000+ banker i Europa via ett API.

## Status
- **Miljö:** SANDBOX → behöver production
- **Credentials:** ✅ Konfigurerade
- **Connector:** `src/connectors/enable_banking.rs`

## Credentials (i ~/.openclaw/secrets/)
| Fil | Innehåll |
|-----|----------|
| `enable-banking.json` | app_id, session-id, konto-UUIDs |
| `enable-banking-key.pem` | RSA private key (2048-bit) |
| `enablebanking-config.json` | Sandbox-config |
| `enablebanking-private.pem` | Sandbox private key |

## Konton vi har access till
| Alias | UUID | Valuta |
|-------|------|--------|
| sek_1 | 65f16d5c-... | SEK |
| eur   | 67333f2a-... | EUR |
| sek_2 | b5f74b06-... | SEK |
| sek_3 | fee7783d-... | SEK |

## API Endpoints
```
GET /sessions/{session_id}/accounts
GET /sessions/{session_id}/accounts/{id}/transactions?date_from=&date_to=
GET /sessions/{session_id}/accounts/{id}/balances
```

## Auth
JWT Bearer, RS256, signerat med private key.
```json
{ "iss": "<app_id>", "iat": <now>, "exp": <now+3600> }
```

## För production
1. Logga in på app.enablebanking.com
2. Byt `environment: SANDBOX` → `PRODUCTION`
3. Ny konsentsession (PSU-redirect flow) per bank

## Dokumentation
https://enablebanking.com/docs/api/reference/
