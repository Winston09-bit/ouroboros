# Kvittovalvet – Kategorisystem

Alla transaktioner kategoriseras automatiskt baserat på merchant.

## Huvudkategorier

| Kod | Svenska | Engelska | Typiska merchants |
|-----|---------|----------|-------------------|
| `DAGLIGVAROR` | Dagligvaror | Groceries | ICA, Coop, Willys, Hemköp, Lidl, City Gross |
| `DRIVMEDEL` | Drivmedel | Fuel/Energy | Circle K, OKQ8, ST1, Preem, Tesla, ChargeNode |
| `BYGG_ELEKTRONIK` | Bygg & Elektronik | Construction/Electronics | Clas Ohlson, Bauhaus, Biltema, Elgiganten, Power |
| `HOTELL_RESOR` | Hotell & Resor | Travel/Hotels | Scandic, Strawberry, SJ, SAS, Booking.com |
| `RESTAURANG` | Restaurang | Restaurants | McDonald's, Max, Espresso House |
| `TRANSPORT` | Transport | Transportation | Bolt, Uber, SL, Västtrafik, Taxi |
| `TELEKOM` | Telekom | Telecom | Telia, Tele2, Tre, Telenor |
| `STREAMING` | Streaming/Mjukvara | Software/Streaming | Netflix, Spotify, Adobe, Microsoft |
| `KONTOR` | Kontor | Office supplies | Staples, IKEA, Office Depot |
| `BANK_FINANS` | Bank & Finans | Banking/Finance | Bank-avgifter, Klarna, Resurs |
| `MEDICIN_VÅRD` | Medicin & Vård | Health/Medical | Apotek, vårdcentraler |
| `KLÄDER` | Kläder | Apparel | H&M, Zara, Lindex |
| `MÖBLER_HEMINREDNING` | Möbler & Heminredning | Furniture | IKEA, Mio, Jysk |
| `UNDERHÅLLNING` | Underhållning | Entertainment | Biograf, evenemang, spelbutiker |
| `UTBILDNING` | Utbildning | Education | Kurser, kursavgifter |
| `LÖN_PERSONAL` | Lön & Personal | Payroll | Löner, sociala avgifter |
| `MOMS` | Moms | VAT | Skatteverket VAT |
| `SKATT` | Skatt | Tax | Skatteinbetalningar |
| `BANK_AVGIFTER` | Bankavgifter | Bank fees | – |
| `RÄNTA` | Ränta | Interest | – |
| `ÖVRIGT` | Övrigt | Other | Fallback |

## MCC-mappning (Merchant Category Code)

MCC från Visa/Mastercard mappas automatiskt:

| MCC Range | Kategori |
|-----------|----------|
| 5411 | DAGLIGVAROR (Grocery Stores) |
| 5541, 5542 | DRIVMEDEL |
| 5200, 5712, 5722, 5732 | BYGG_ELEKTRONIK |
| 7011, 4722 | HOTELL_RESOR |
| 5812, 5813, 5814 | RESTAURANG |
| 4111, 4121, 4131 | TRANSPORT |
| 4814, 4899 | TELEKOM |
| 5815, 5816, 5817, 5818 | STREAMING |
| 5943, 5944 | KONTOR |
| 6010, 6011, 6012 | BANK_FINANS |

## SE BAS-kontoplan mappning

Mot svenska bokföringskonton:

| Kategori | BAS-konto |
|----------|-----------|
| DAGLIGVAROR | 6982 (Mat och dryck) |
| DRIVMEDEL | 5611 (Drivmedel) |
| BYGG_ELEKTRONIK | 5410 (Förbrukn.inv) eller 1220 (Inv) |
| HOTELL_RESOR | 5810 (Hotell), 5800 (Resor) |
| RESTAURANG | 6071 (Repr. avdragsgill) |
| TRANSPORT | 5500 (Reparation/transport) |
| TELEKOM | 6212 (Telefoni) |
| STREAMING | 6230 (Programvaror) |
| KONTOR | 6110 (Kontorsmaterial) |
| BANK_AVGIFTER | 6570 (Bankkostnader) |

## Confidence

Varje kategorisering har confidence 0.0-1.0:
- 1.0: Direkt MCC-match
- 0.9: Alias-match från MerchantResolver
- 0.7: Substring-match på description
- 0.5: AI-fallback (LLM klassificering)
- 0.0: Okategoriserad → ÖVRIGT
