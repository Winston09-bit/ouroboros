# KNOWN LIMITATIONS — Allied Properties (Phase 1)

Honest status of what is and isn't done. Nothing here is a hidden defect — these
are scope boundaries or items awaiting client input.

## Awaiting client input (blockers for "final")
- **Visual design = placeholder palette/type.** The design system is centralized
  in `assets/css/tokens.css` with institutional placeholder values. The exact
  palette, typography, and spacing apply once the **homepage mockup** is
  provided. See `docs/MOCKUP-INTEGRATION-PLAN.md`.
- **Fonts not bundled.** Display/body fonts fall back to Georgia/Arial until the
  licensed fonts are added and self-hosted (no CDN, per the no-external rule).
- **Photography & logos.** Aerial/site imagery, leadership photos, and partner
  logos are placeholders until supplied.
- **Real copy.** Section copy on Home/Capabilities/etc. is professional
  placeholder text pending client-approved wording.

## Intentional scope boundaries (Phase 2 / not this phase)
- **No live data feeds.** Portals are manually maintained gated areas (documents,
  lists, PDFs) — no CRM/Procore/SharePoint/API integration. By design.
- **No external APIs / third-party developer dependencies.**

## Implementation notes / minor
- **Leadership & homepage hero copy** are currently in template code, not custom
  fields. Editable by the developer; can be promoted to ACF/Customizer fields if
  the client expects to edit them frequently (small follow-up, flagged in
  ADMIN-GUIDE).
- **Forms via plugin (recommended).** A working no-plugin fallback ships for the
  landowner form; the production path is CF7/WPForms. Email deliverability
  requires an SMTP plugin on most hosts.
- **Portfolio filter** uses progressive enhancement: JS filters instantly;
  without JS the buttons are real region-archive links (still functional).
- **No comments** on pages/communities (institutional site — intentional).

## Verified working (QA passed)
- All 16 routes return correct HTTP status; 0 broken internal links.
- Landowner form: validation + email notification + file-upload handling.
- Portals: login, role-based access, access-denied, document listing.
- No PHP warnings/notices from the theme across a full authenticated crawl.
- Responsive rules present; accessible base (skip link, focus, aria, lang).
- Basic SEO: title tag, meta description (with fallback), Open Graph, Twitter card.

## Environment caveat for reviewers
- Pixel screenshots could not be generated in the build sandbox (the headless
  browser binary is blocked by network policy). Self-contained HTML snapshots
  were provided instead; true screenshots come from any host/LocalWP/XAMPP.
- PHP's built-in dev server returns 404 on multipart POST to extensionless URLs
  (a `php -S` quirk); on real Apache/nginx the upload form posts normally. The
  form handler logic was verified directly.
