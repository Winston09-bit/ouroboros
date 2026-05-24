# Revolut Business API

## Status
- **Miljö:** PRODUCTION
- **Connector:** `src/connectors/revolut_impl.rs`
- **Credentials:** ✅ Alla på plats

## ✅ Auth fungerar – behöver IP-whitelist

**Status 2026-05-24:** OAuth + refresh-token-flödet funkar.
```
{"access_token":"oa_prod_...","token_type":"bearer","expires_in":2399}
```

**Nuvarande fel vid faktiskt API-anrop:**
```
403 Forbidden: IP address not whitelisted. 
Verify IP whitelist configuration in Revolut Business Portal.
```

### Action: Lägg till IP i whitelist
1. Logga in på https://business.revolut.com
2. Settings → APIs → vår app
3. "IP whitelist" → lägg till server-IP
4. För utveckling: lägg till Winston's WSL public IP
5. För production: AWS Elastic IP

Kolla nuvarande IP: `curl -s ifconfig.me`

## OAuth flow (verifierat fungerar)

### Refresh-token-grant (det vi använder nu)
```bash
curl -X POST https://b2b.revolut.com/api/1.0/auth/token \
  -d "grant_type=refresh_token" \
  -d "refresh_token=oa_prod_..." \
  -d "client_assertion_type=urn:ietf:params:oauth:client-assertion-type:jwt-bearer" \
  -d "client_assertion=<JWT signerat med revolut-private.pem>"
```

### JWT-format för client_assertion
```json
{
  "iss": "api.wavult.com",          // domain utan https://
  "sub": "<client_id>",
  "aud": "https://revolut.com",
  "iat": <now>,
  "exp": <now + 3600>
}
```
Sign: RS256 med revolut-private.pem

## API-endpoints (när auth funkar)
```
GET /api/1.0/accounts
GET /api/1.0/transactions?from=&to=&count=1000
GET /api/1.0/transaction/{id}
GET /api/1.0/counterparties
```
