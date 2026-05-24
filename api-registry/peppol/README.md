# Peppol

> Europeisk standard för e-fakturering. Krävs för B2B och offentlig sektor.

## Status
- **Status:** ❌ Saknar accesspunkt
- **Prioritet:** P1

## Vad Peppol är
Peppol är ett nätverk av accesspunkter. För att skicka/ta emot Peppol-fakturor behöver vi:
1. En **Peppol-accesspunkt** (AP) – en godkänd leverantör
2. Registrering i **SMP/SML** – Peppol-katalogen

## Rekommenderade accesspunkt-leverantörer (Sverige)
| Leverantör | Pris/mån | Kommentar |
|------------|----------|-----------|
| **Inexchange** | ~500 kr | Enklast för SME |
| **Pagero** | ~800 kr | Enterprise-fokus |
| **Netnordic** | ~600 kr | Bra support |
| **OpusCapita** | ~700 kr | Nordic fokus |

## Åtgärd
1. Kontakta Inexchange (inexchange.se/peppol)
2. Ange: org.nr 559141-7042 (LandveX AB)
3. Välj API-access (inte bara webb-portal)
4. Kostnad: ~500 kr/mån

## Peppol ID format
```
0007:5591417042   (0007 = Swedish org.nr, sedan org.nr utan bindestreck)
```

## Vad vi får
- Ta emot Peppol BIS Billing 3.0-fakturor automatiskt
- Skicka fakturor i Peppol-format
- Direkt integration med leverantörers system
- Metadata: fakturanummer, belopp, moms, betalningsvillkor

## Dokumentation
https://docs.peppol.eu/poacc/billing/3.0/
https://www.inexchange.se/developer
