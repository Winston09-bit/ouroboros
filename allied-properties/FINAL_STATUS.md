# FINAL_STATUS — Allied Properties (Phase 1)

Acceptance pass run against a **running** local install (WordPress 6.5.8 / PHP 8.4,
SQLite). Verified by execution, not by reading code. Last updated for Phase 8.

Legend: ✅ verified running · ⚠️ client-required/placeholder · ❌ not done ·
🔬 EJ VERIFIERAT (couldn't be proven in this environment)

---

## KLART (verified ✅)

### Routes / pages
- Home, The Firm, Capabilities, Communities (archive), Single community,
  Partners, Sell Us Your Land, Contact, Portals hub, Portal pages, wp-admin →
  **all HTTP 200**, warm response **23–35 ms**, no broken assets.
- 404 returns branded page; search works; region taxonomy archive works.

### Interaction / buttons
- 37 links, 3 buttons, 2 forms inventoried. **No `fetch`/AJAX/XHR and no
  server-side external calls anywhere in the theme** → nothing can "hang".
- JS (nav toggle + portfolio filter) loads non-blocking in the footer; DOM
  targets present; filter also works without JS (real region-archive links).

### Landowner form (handler logic)
- Empty / missing email / invalid email → **error, no email sent**. ✅
- Valid submit → **success + email triggered** (captured via `pre_wp_mail`). ✅
- Wrong nonce → **security error**. ✅  Honeypot filled → silent success, no mail. ✅
- File MIME allowlist: jpg/png/webp/pdf **allowed**; exe/php/txt **rejected**. ✅

### Portals (access matrix, real logins)
| role \ portal | builder | investor | partner | bad-slug |
|---|---|---|---|---|
| guest | login | login | login | unavailable |
| builder | **access** | denied | denied | unavailable |
| investor | denied | **access** | denied | unavailable |
| partner | denied | denied | **access** | unavailable |
| admin | access | access | access | **admin warning** |
- Logout clears the session → portal requires login again. ✅
- Misconfigured slug now shows an **admin warning** and **fails closed** for
  visitors (replaced the previous silent fallback). ✅

### Quality
- `WP_DEBUG` + `SCRIPT_DEBUG` on, full authenticated crawl → **0 theme
  warnings/notices/undefined/fatals**. ✅
- Performance: dev server runs with 4 workers + cron disabled →
  8 parallel requests **0.239 s → 0.104 s**, worst single **217 ms → 77 ms**. ✅
- Emoji/embed/head clutter removed (no external `s.w.org` call); front page =
  **1 script + 6 stylesheets**. ✅
- One-command reproducible preview (`preview/start.sh`) verified from a clean
  state. ✅

---

## EJ KLART / KUNDBEROENDEN (⚠️ client-required — NOT bugs)
- **Approved homepage mockup** → exact palette/typography. (design = placeholder)
- **Licensed fonts** (files) — currently Georgia/Arial fallback.
- **Photography**: hero, community, leadership photos (placeholders).
- **Partner logos** (logo walls are placeholders).
- **Final approved copy** for Home/Capabilities/Firm/etc.
- **Production lead-email inbox** + SMTP plugin choice.
- **Form plugin choice** (CF7 vs WPForms) for the production form.

---

## RISKER / EJ VERIFIERAT (🔬)
- 🔬 **Real multipart file upload over HTTP**: PHP's built-in dev server returns
  inconsistent 404s on multipart POST (a `php -S` limitation). The handler logic
  (validation, MIME allowlist, email) is verified directly; the actual file
  *move* uses WordPress core `wp_handle_upload` and runs normally on Apache/nginx
  — **not verifiable on the dev server**. Production path is CF7/WPForms anyway.
- 🔬 **Actual email delivery (SMTP)**: `wp_mail()` is invoked correctly with the
  right recipient/subject/body/attachments (captured), but no real mail server
  exists here. Requires WP Mail SMTP on the host — **delivery not verified**.
- 🔬 **LCP / TTI / CLS**: require a real browser; the headless browser is blocked
  in this environment. TTFB/total time measured instead (fast). Layout-shift risk
  is low (images use fixed `aspect-ratio`), but **Core Web Vitals not measured**.
- 🔬 **Cross-browser/visual screenshots**: provided as standalone HTML snapshots;
  pixel screenshots require running locally.

---

## GO / NO-GO
**GO for the Phase 1 build.** Every route, the lead-form logic, the portal
access control, logout/session, and performance are verified on a running site
with zero theme warnings. No clickable element can hang. Remaining items are
**client-supplied assets** (mockup, fonts, photos, copy) and host-side config
(SMTP) — none are code defects.

A client can click around for 20 minutes without hitting an error or a dead
button. The site is **not yet launch-final** only because the visual design and
content are awaiting the mockup and assets.
