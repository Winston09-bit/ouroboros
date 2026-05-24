# Nordea Open Banking

## Status
- **Miljö:** SANDBOX
- **Credentials:** ✅ client_id + client_secret
- **Connector:** `src/connectors/nordea.rs` (stub → behöver implementation)

## Credentials
| Fil | Innehåll |
|-----|----------|
| `nordea-api-credentials.json` | client_id, client_secret, sandbox |

## OAuth2 flow (PSD2)
```
1. GET https://api.nordeaopenbanking.com/v2/authorize
   ?client_id=<id>&redirect_uri=<uri>&scope=ACCOUNTS_BASIC,TRANSACTIONS_BASIC&response_type=code

2. POST /v2/authentication/access_token
   client_id=<id>&client_secret=<secret>&grant_type=authorization_code&code=<code>&redirect_uri=<uri>

3. Svar: { access_token, token_type, expires_in, refresh_token }
```

## API Endpoints
```
GET /v4/accounts                           – lista konton
GET /v4/accounts/{id}/transactions         – transaktioner
GET /v4/accounts/{id}/transactions/{txId}  – enskild transaktion
```

## För production
1. Logga in på developer.nordeaopenbanking.com
2. Ansök om production-access
3. Verifiering tar 2-4 veckor

## Dokumentation
https://developer.nordeaopenbanking.com/
