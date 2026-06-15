# Allied Properties — WordPress Theme (Phase 1)

Institutional marketing site and gated portals for **Allied Properties**, a
residential land development firm operating across Northeastern North Carolina
and Hampton Roads, Virginia.

This repository contains a **self-contained custom WordPress theme** (`allied`)
plus handoff documentation. It has **no external API or third-party developer
dependencies** — live data feeds are an explicitly separate later phase.

> **Design status:** The structure, page templates, custom post types, lead
> form, and gated portals are complete. The visual layer is driven by a single
> design-token file (`assets/css/tokens.css`) using **institutional placeholder
> values**. Once the approved homepage mockup is provided, the exact palette,
> typography, and spacing are applied by editing that one file (plus dropping in
> self-hosted fonts). No structural rework is needed.

---

## What's included (Phase 1 scope)

**Public marketing site**
- **Home** — `front-page.php` (hero, positioning, proof stats, capabilities,
  featured communities, partner logo wall, CTA)
- **The Firm / About + Leadership** — `page-templates/template-firm.php`
- **Capabilities** — `page-templates/template-capabilities.php`
- **Communities / Portfolio** — `archive-community.php` (filterable grid) +
  `single-community.php` (individual project template)
- **Partners** — `page-templates/template-partners.php`
- **Landowners — "Sell us your land"** — `page-templates/template-landowner.php`
  (working lead form: name, contact, parcel info, file/photo upload → email)
- **Contact** — `page-templates/template-contact.php`

**Gated portals (simple, manually maintained — no live data)**
- **Portals hub** — `page-templates/template-portals.php`
- **Builder / Investor / Partner portals** — `page-templates/template-portal.php`
  (one template, gated by page slug → custom role; hosts documents/PDFs/lists)

**Foundations**
- Custom post type **Community** with `region` + `project_status` taxonomies and
  project meta (location, acreage, lots, builder, year) — no plugin required.
- Three custom roles (`allied_builder`, `allied_investor`, `allied_partner`)
  with capability-based access control.
- Centralized design-token CSS, responsive, accessible (skip links, focus
  styles, reduced-motion, semantic landmarks), basic on-page SEO + Open Graph,
  block-editor color/font presets (`theme.json`), and starter content.

---

## Installation

1. Copy `wp-content/themes/allied/` into your WordPress install's
   `wp-content/themes/` directory (or upload the zipped theme via
   **Appearance → Themes → Add New → Upload Theme**).
2. **Activate** the *Allied Properties* theme. Activation registers the
   Community post type and the three portal roles.
3. On a fresh site, visit **Appearance → Customize** to apply **starter
   content** (creates all core pages with the right templates + a primary menu).
   On an existing site, create the pages manually — see `HANDOFF.md`.
4. Set a static homepage: **Settings → Reading → Your homepage displays → A
   static page → Home**.
5. Go to **Settings → Permalinks** and click **Save** (registers the
   `/communities/` and `/portals/...` URLs).
6. Install the recommended plugins for the forms (see below), add content, and
   replace placeholder imagery with supplied photography.

## Recommended plugins (standard, per the agreed approach)

- **Contact Form 7** or **WPForms Lite** — landowner + contact forms. The
  landowner template auto-detects a form shortcode and hides its built-in
  fallback form when one is present. (The fallback native form works out of the
  box if you prefer zero plugins.)
- **Members** or **User Role Editor** — manage portal roles/users via a UI.
- *(Optional)* **Yoast SEO** / **Rank Math** — richer SEO. The theme ships
  sensible meta-description + Open Graph defaults so this is optional.
- *(Recommended for email deliverability)* an SMTP plugin (e.g. **WP Mail SMTP**)
  so lead notifications reliably reach your inbox.

## Local preview (optional)

A `docker-compose.yml` is provided to spin up WordPress + MySQL with this theme
mounted, so you can preview before deploying to your host:

```bash
cd allied-properties
docker compose up -d
# WordPress: http://localhost:8088  — run the installer, then activate "Allied Properties"
```

See **`HANDOFF.md`** for the non-technical content-editing guide and the exact
form/portal setup steps.
