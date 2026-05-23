# Revisionsrav — Wavult OS
**Version:** 1.0.0  
**Datum:** 2026-05-23  
**Utfärdare:** LandveX AB (org.nr 559141-7042)  
**Status:** UTKAST — Framtida standard  
**Målgrupp:** Intern IT-revision, externa revisorer, Skatteverket

---

## Syfte

Detta dokument definierar minimikraven för systemrevision av Wavult OS och dess integrerade finansiella infrastruktur. Kraven är utformade för att möta svenska och EU-rättsliga krav på finansiell spårbarhet, dataintegritet och systemsäkerhet — inklusive Skatteverkets krav på elektronisk bokföring och revisionsspår.

---

## 1. Systemöversikt

| Komponent | URL | Status (2026-05-23) |
|-----------|-----|---------------------|
| Wavult OS (frontend) | os.wavult.com | ✅ Operationell |
| Wavult API | api.wavult.com | ⚠️ Degraderad |
| AMOS-server | 16.170.83.169 (eu-north-1) | ⚠️ Begränsad åtkomst |
| Bernt (AI-infrastruktur) | bernt.wavult.com | ✅ Operationell |
| Databas | PostgreSQL (primär) | ✅ OK |

---

## 2. Revisionskrav — Säkerhet & Åtkomst

### RAV-S1: SSH-åtkomst och nyckelhantering
- **Krav:** Alla serveranslutningar ska ske via SSH med ed25519-nycklar. Lösenordsinloggning ska vara inaktiverat.
- **Krav:** SSH-nycklar ska vara registrerade per användare i en central nyckelförteckning.
- **Krav:** fail2ban eller motsvarande intrångsskydd ska vara aktiverat och konfigurerat med max 3 inloggningsförsök.
- **Krav:** Port 22 ska vara begränsad till specifika IP-adresser via AWS Security Group — aldrig öppen mot 0.0.0.0/0.
- **Nuläge (2026-05-23):** ❌ Port 22 stängd/oåtkomlig. Nyckelhantering ej dokumenterad.
- **Åtgärd:** Johan Berglund (CTO) ansvarar för att korrigera security group-konfiguration.

### RAV-S2: JWT-autentisering
- **Krav:** JWT-tokens ska signeras med RS256 (asymmetrisk kryptering), inte HS256 (symmetrisk).
- **Krav:** Token-validering ska fungera konsekvent — login-endpoint och API-endpoints ska dela samma JWT-secret/publik nyckel.
- **Krav:** Token-livslängd max 600 sekunder (access token), 30 dagar (refresh token).
- **Nuläge (2026-05-23):** ❌ Tokens genereras men avvisas av API — JWT_SECRET-mismatch misstänkt.
- **Åtgärd:** Verifiera att `identity.wavult.com` och `api.wavult.com` delar samma JWT-valideringsnyckel.

### RAV-S3: API-endpoints
- **Krav:** Alla `/v1/*` endpoints ska kräva giltig autentisering (401 på oautentiserade anrop).
- **Krav:** `/health` ska vara publik men inte exponera intern information.
- **Nuläge (2026-05-23):** ✅ 401-skydd på plats. ⚠️ `/health` exponerar `supabase_client: fallback` — bör åtgärdas.

---

## 3. Revisionskrav — Finansiell Integritet

### RAV-F1: Bankintegrationer (Open Banking / PSD2)
- **Krav:** Alla bankintegrationer ska använda godkända PSD2/Open Banking-protokoll.
- **Krav:** Nordea-koppling ska vara i produktionsläge (ej sandbox) innan lansering.
- **Krav:** OAuth-consent-flödet ska spara anslutningsstatus atomärt — status ska uppdateras i samma transaktion som consent bekräftas.
- **Nuläge (2026-05-23):** ❌ Nordea-connector är en stub utan riktiga API-anrop. Environment = `sandbox`.
- **Åtgärd:** Implementera fullt OAuth-flöde mot Nordea Open Banking production. Aktivera credentials.

### RAV-F2: Dubbelsidig bokföring (Double-Entry Ledger)
- **Krav:** Varje transaktion ska generera minst två verifikationsposter (debet/kredit) i enlighet med BAS-kontoplanen 2024.
- **Krav:** Ledger-saldo ska alltid balansera — Σ Debet = Σ Kredit.
- **Krav:** Ingen post får raderas — endast reversering via motkontring.
- **Nuläge:** Implementerat i Reconciler-kärnan (reconciler-core/src/).

### RAV-F3: Momshantering
- **Krav:** Moms ska beräknas och verifieras automatiskt för varje transaktion.
- **Krav:** Systemet ska stödja momsrapportering enligt Skatteverkets SRU-format (skattedeklaration via fil).
- **Krav:** Alla momssatser (0%, 6%, 12%, 25%) ska hanteras korrekt per transaktionstyp.
- **Nuläge:** Grundläggande momsberäkning implementerad. SRU-export ej implementerad.
- **Åtgärd:** Implementera SRU-filgenerering för momsdeklaration.

### RAV-F4: Revisionsspår (Audit Trail)
- **Krav:** Varje händelse ska loggas med: timestamp, användar-ID, IP-adress, åtgärd, före/efter-värde.
- **Krav:** Audit-loggar ska vara immutabla — ingen borttagningsmöjlighet.
- **Krav:** Loggar ska bevaras i minst 7 år (Bokföringslagen 7:2).
- **Krav:** Data Lineage ska kunna spåra varje transaktion från ursprungskälla till bokförd post.
- **Nuläge:** Data lineage-modul implementerad i reconciler-core.

---

## 4. Revisionskrav — Systemtillgänglighet

### RAV-T1: Hälsokontroll (Health Check)
- **Krav:** `/health` ska returnera `status: ok` med alla beroenden listade.
- **Krav:** Om något beroende är degraderat ska det tydligt framgå med orsak.
- **Krav:** Felaktiga/oanvända beroenden ska inte rapporteras i health-endpointen.
- **Nuläge (2026-05-23):** ⚠️ `supabase_client: fallback` rapporteras trots att Supabase aldrig använts. Vilseledande.
- **Åtgärd:** Ta bort supabase_client-check från health-endpoint.

### RAV-T2: Autostart och driftsäkerhet
- **Krav:** Alla tjänster ska startas automatiskt vid serveromstart via systemd eller motsvarande.
- **Krav:** Demo- och utvecklingsservrar ska inte köras som permanenta processer i produktion.
- **Nuläge:** ⚠️ Demo (localhost:9977) startas manuellt — dör vid omstart.
- **Åtgärd:** Konfigurera systemd-service för demo eller ta bort från produktion.

### RAV-T3: Databasåtkomst
- **Krav:** PostgreSQL ska vara primär databas (ej in-memory) i produktion.
- **Krav:** Databas-migrations ska köras automatiskt vid deploy.
- **Nuläge:** ⚠️ In-memory möjligen fortfarande aktiv som fallback.
- **Åtgärd:** Verifiera att PostgreSQL är default och att migrations är körda.

---

## 5. Revisionskrav — Skatteverket-kompatibilitet

### RAV-SKV1: SRU-export (Momsdeklaration)
- **Krav:** Systemet ska kunna exportera momsdeklaration i SRU-format enligt Skatteverkets specifikation.
- **Deadline:** Före produktion.

### RAV-SKV2: SIE-export (Bokföringsdata)
- **Krav:** Systemet ska stödja SIE4-format för export av bokföringsdata (branschstandard för revision i Sverige).
- **Deadline:** Före extern revision.

### RAV-SKV3: Organisationsnummer och registrering
- **Krav:** Organisationsnummer (559141-7042) ska vara korrekt registrerat i alla transaktioner.
- **Krav:** Momsnummer (SE559141704201) ska vara korrekt på alla fakturor.

### RAV-SKV4: Bokföringslagens krav
- **Krav:** Bokföring ska ske löpande och avslutas senast inom 2 månader efter räkenskapsperiodens slut.
- **Krav:** Verifikationer ska vara numrerade i löpande följd och inte kunna ändras efter attestering.

---

## 6. Öppna Åtgärdspunkter (Prioritetsordning)

| # | Krav | Ansvarig | Prioritet | Status |
|---|------|----------|-----------|--------|
| 1 | JWT-secret konfigureras korrekt på api.wavult.com | Johan (CTO) | 🔴 AKUT | ❌ Öppen |
| 2 | SSH-åtkomst till AMOS öppnas och dokumenteras | Johan (CTO) | 🔴 AKUT | ❌ Öppen |
| 3 | supabase_client tas bort från health-endpoint | Johan (CTO) | 🟡 Hög | ❌ Öppen |
| 4 | PostgreSQL som default verifieras | Johan (CTO) | 🟡 Hög | ❌ Öppen |
| 5 | Nordea-connector implementeras (production) | Dev | 🟡 Hög | ❌ Öppen |
| 6 | SRU-export implementeras | Dev | 🟡 Hög | ❌ Öppen |
| 7 | SIE4-export implementeras | Dev | 🟠 Medium | ❌ Öppen |
| 8 | BankID/eIDAS-integration | Dev | 🟠 Medium | ❌ Öppen |
| 9 | Tenant crypto-isolation | Dev | 🟠 Medium | ❌ Öppen |
| 10 | Autostart-konfiguration för alla tjänster | Johan (CTO) | 🟢 Låg | ❌ Öppen |

---

## 7. Signering och Godkännande

| Roll | Namn | Datum | Signatur |
|------|------|-------|----------|
| CFO / Ansvarig | Winston Bjarnemark | 2026-05-23 | ____________ |
| CTO / Tekniskt ansvarig | Johan Berglund | __________ | ____________ |
| Extern revisor | __________________ | __________ | ____________ |

---

*Dokumentet ska revideras kvartalsvis eller vid väsentliga systemförändringar.*  
*Nästa revision: 2026-08-23*
