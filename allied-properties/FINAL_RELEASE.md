# FINAL_RELEASE — Allied Properties (Phase 1 + Phase 9 hardening)

Status of the build for delivery. Functionality is frozen; Phase 9 was
production readiness + UX hardening in the existing design language (no new
features, no structural rebuild, URLs and content model preserved).

## VERIFIED (run on a live install, WP 6.5.8 / PHP 8.4)
- **All routes 200**, branded 404, search works. 0 PHP warnings/notices with
  `WP_DEBUG` + `SCRIPT_DEBUG` across an authenticated crawl.
- **Lead form** handler: validation (empty/invalid/nonce/honeypot), MIME
  allowlist (jpg/png/webp/pdf in; exe/php/txt out), email triggered on success.
- **Portals**: access matrix correct for guest/builder/investor/partner/admin;
  logout clears session; unknown slug fails closed with an admin warning.
- **Interaction**: no `fetch`/AJAX/external call anywhere in the theme — no
  control can hang; all feedback is immediate (≤200 ms hover, `:active` press).
- **Accessibility**: skip link, `lang`, nav `aria-label`/`aria-controls`/
  `aria-expanded`, `aria-pressed` filters, visible `:focus-visible`, all images
  carry alt, 44px mobile tap targets, `prefers-reduced-motion` respected.
- **Mobile**: menu locks scroll, closes on tap/Escape/resize.
- **Performance**: warm responses ≈25 ms; dev server runs 4 workers + cron off;
  emoji/embed/head clutter removed; front page = 1 script + 6 stylesheets.
- **States**: branded empty states (home/archive/search), distinct success/error
  form states, loading skeleton on card media.
- **Reproducible preview**: `preview/start.sh` boots from a clean state to a
  working `http://localhost:8088`.

## ASSUMED (reasonable defaults; verify on the production host)
- **Email delivery** works once an SMTP plugin (e.g. WP Mail SMTP) is configured
  — `wp_mail()` is called correctly; actual delivery not testable here.
- **Real multipart upload** works on Apache/nginx (verified logic, not the
  `php -S` transport).
- Core Web Vitals are good (lightweight, low CLS risk) — not formally measured
  without a browser.

## CUSTOMER CONTENT ONLY (not code; supply when ready)
- Homepage **mockup** / final palette + **licensed fonts** (design tokens are
  ready to swap — see `docs/MOCKUP-INTEGRATION-PLAN.md`).
- **Photography** (hero, communities, leadership) and **partner logos**.
- **Final approved copy** and any **real proof-bar figures** (lots/acres) —
  current copy is credible positioning, no invented metrics.
- **Lead-email inbox** + choice of form plugin (CF7 / WPForms).

## DECISIONS MADE THIS PHASE (no design assets needed)
- Tightened type scale + hierarchy, spacing rhythm, eyebrow detail.
- Snappy interaction feedback; refined buttons/cards/nav/filters.
- Branded empty + success/error states; graceful public messaging (no admin
  copy leaking to visitors).
- Default on-brand favicon (used only until a Site Icon is set).
- Kept the palette fully token-swappable for an eventual mockup.

## GO / NO-GO
**GO.** The site is production-ready on the engineering and UX axes: nothing
hangs, nothing errors, states are handled, it's accessible and responsive, and
it's editable by a non-technical admin. The only remaining items are
**customer-supplied content/branding** and **host-side email config** — neither
is a code defect. See `docs/BUG_REPORT.md` for the QA detail and
`docs/DEPLOYMENT-CHECKLIST.md` before launch.
