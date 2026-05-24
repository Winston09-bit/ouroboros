# Fortnox

> Viktigaste ERP-integrationen. ~40% av svenska SMEs använder Fortnox.

## Status
- **Miljö:** ❌ Saknar credentials
- **Prioritet:** P0 – BLOCK för ERP-integration

## Vad vi behöver göra

### Steg 1: Registrera Fortnox-app (Winston gör detta)
1. Gå till https://apps.fortnox.se/integrations/
2. Klicka "Skapa ny integration"
3. Fyll i:
   - Namn: Kvittovalvet
   - Redirect URI: https://os.wavult.com/fortnox/callback (eller localhost för dev)
   - Scopes: **Bokföring, Faktura, Leverantörsfaktura, Verifikation, Fil**
4. Spara → få `client_id` + `client_secret`

### Steg 2: OAuth2 flow
```
1. Redirect → https://apps.fortnox.se/oauth-v1/auth
   ?client_id=<id>&redirect_uri=<uri>&scope=bookkeeping+invoice&state=<random>&response_type=code&access_type=offline

2. Användaren godkänner

3. Callback med ?code=<code>

4. POST https://api.fortnox.se/oauth-v1/token
   grant_type=authorization_code
   &code=<code>
   &client_id=<id>
   &client_secret=<secret>
   &redirect_uri=<uri>

5. Svar: { access_token, refresh_token, token_type }
```

### Steg 3: Spara i secrets
```bash
cat > ~/.openclaw/secrets/fortnox.json << EOF
{
  "client_id": "<ID>",
  "client_secret": "<SECRET>",
  "access_token": "<TOKEN>",
  "refresh_token": "<REFRESH>",
  "environment": "production"
}
EOF
```

## API Endpoints vi behöver
```
GET  /3/vouchers               – verifikationer
GET  /3/invoices               – fakturor (kund)
GET  /3/supplierinvoices       – leverantörsfakturor
GET  /3/accounts               – kontoplan
POST /3/vouchers               – skapa verifikation
GET  /3/attachments/connect/{entitytype}/{entityId} – bilagor
```

## Dokumentation
https://developer.fortnox.se/documentation/
