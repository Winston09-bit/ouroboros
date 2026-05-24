# Revolut Business API

## Status
- **Miljö:** Oklart (verifieras)
- **Credentials:** ✅ client_id + refresh_token + RSA private key
- **Connector:** `src/connectors/revolut_impl.rs`
- **Token expires:** 2031-03-28

## Credentials
| Fil | Innehåll |
|-----|----------|
| `revolut-business-api.json` | client_id, refresh_token, jwt_issuer |
| `revolut-private.pem` | RSA private key (4096-bit) |
| `revolut-rufus-private.pem` | Rufus-specifik nyckel |
| `revolut-rufus-public.pem` | Publik nyckel för Rufus |

## Auth flow
```
1. Generera JWT:
   { iss: client_id, sub: client_id, aud: "https://revolut.com", iat, exp }
   Signera med RS256 + revolut-private.pem

2. POST /auth/token:
   grant_type=refresh_token
   &refresh_token=<token>
   &client_assertion_type=urn:ietf:params:oauth:client-assertion-type:jwt-bearer
   &client_assertion=<JWT>

3. Svar: { access_token, token_type: "Bearer", expires_in }
```

## API Endpoints
```
GET  /accounts              – lista konton
GET  /transactions          – alla transaktioner (?from=&to=&count=1000)
GET  /transaction/{id}      – enskild transaktion
POST /pay                   – initiera betalning
GET  /exchange-rates        – valutakurser
```

## Dokumentation
https://developer.revolut.com/docs/business/business-api
