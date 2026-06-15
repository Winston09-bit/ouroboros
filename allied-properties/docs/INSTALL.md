# INSTALL — Allied Properties (Phase 1)

Step-by-step install on your WordPress hosting. ~20 minutes.

## Prerequisites
- WordPress 6.4+ on PHP 8.0+ (tested on WP 6.5.8 / PHP 8.4)
- Ability to upload a theme (admin) and install plugins
- Outgoing email working on the host (or an SMTP plugin — see below)

## 1. Install the theme
**Option A (zip upload):** zip the `allied` folder so the archive contains
`allied/style.css` at its root, then **Appearance → Themes → Add New → Upload
Theme** → choose the zip → **Install** → **Activate**.

**Option B (SFTP):** copy `wp-content/themes/allied/` into your install's
`wp-content/themes/` directory, then **Appearance → Themes → Activate**.

> Activation auto-registers the **Community** post type and the three portal
> roles (Builder/Investor/Partner Member).

## 2. Apply page structure
- **Fresh site:** open **Appearance → Customize** once to trigger **starter
  content** (creates all pages with the correct templates + a primary menu),
  then **Publish**.
- **Existing site / starter content already dismissed:** create the pages
  manually per the table in `docs/ADMIN-GUIDE.md` (§2) and assign each page its
  template.

## 3. Set the homepage
**Settings → Reading → Your homepage displays → A static page → Homepage = Home.**

## 4. Flush permalinks (important)
**Settings → Permalinks → set "Post name" → Save.** This registers the
`/communities/` archive, single-project URLs, and `/portals/...` URLs.
(If `/communities/` ever 404s, re-save permalinks.)

## 5. Install recommended plugins
- **Contact Form 7** *or* **WPForms Lite** — landowner + contact forms.
- **Members** *or* **User Role Editor** — manage portal users/roles via UI.
- **WP Mail SMTP** — reliable lead-email delivery (strongly recommended).
- *(Optional)* **Yoast SEO** / **Rank Math** — the theme already outputs
  meta-description + Open Graph defaults, so this is optional.

## 6. Forms
See `docs/ADMIN-GUIDE.md` §4. The Landowner page works **out of the box** with a
built-in form (validates, accepts JPG/PNG/PDF upload, emails the admin). It
automatically hides that form and uses your plugin form instead if you paste a
CF7/WPForms shortcode into the page.

## 7. Branding
- **Appearance → Customize → Site Identity:** logo + **Site Icon** (favicon,
  512×512 PNG) + tagline (shows in footer).

## 8. Verify
Run through `docs/DEPLOYMENT-CHECKLIST.md` before going live.
