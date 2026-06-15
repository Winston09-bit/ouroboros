# Allied Properties — Editor & Handoff Guide

A practical guide for a **non-technical admin** to run the site day-to-day, plus
the one-time setup steps. (A short Loom walkthrough should accompany final
handoff — script outline at the bottom.)

---

## 1. Logging in

- Admin dashboard: `https://YOURDOMAIN/wp-admin`
- Use the administrator credentials provided at handoff. Change the password on
  first login (**Users → Profile**).

## 2. Editing page content

Every public page is editable under **Pages**. Click a page → edit text in the
block editor → **Update**. The brand fonts and colors are preloaded in the
editor (the "Display Serif", "Body Sans", and the navy/brass color swatches).

- **The Firm, Capabilities, Partners, Contact, Landowners, Portals** each use a
  custom template (already assigned). The page banner uses the page **title**;
  the intro line uses the page **Excerpt** (open the *Excerpt* panel in the
  right sidebar — enable it under the three-dot menu → *Preferences → Panels* if
  hidden).
- The **Home** page layout lives in the theme (`front-page.php`). Headline and
  section copy that aren't yet wired to fields are edited there by the developer;
  most clients edit Home copy rarely. Ask the developer to move any specific Home
  text into editable fields if you'll change it often.

## 3. Communities / Portfolio (the project grid)

1. **Communities → Add Community.**
2. Add a **title**, write the description in the body, set a **Featured image**
   (this is the card + hero photo).
3. Fill the **Project Details** box (location, acreage, lots, builder, year).
4. Assign a **Region** and a **Status** (e.g. *In Development*, *Delivered*) —
   these power the filter buttons on the Communities page.
5. Add extra photos by attaching images to the community (Media → upload while
   editing) to populate the project gallery.
6. **Publish.** It appears automatically in the grid and on the homepage
   "Selected communities" section.

## 4. Landowner lead form (email notifications)

**Recommended (plugin) setup with Contact Form 7:**
1. Install & activate **Contact Form 7**.
2. **Contact → Add New**. Suggested fields:
   - Name (required), Email (required), Phone, County/City, Parcel ID/Address,
     Approx. Acreage, Message, and a **file upload** field:
     `[file parcel-file limit:10mb filetypes:jpg|jpeg|png|webp|pdf]`
3. On the **Mail** tab, set recipient to your acquisitions inbox and add
   `[parcel-file]` under *File attachments*.
4. Copy the form's shortcode, edit the **Sell Us Your Land** page, paste the
   shortcode into the form column, **Update**. The page automatically hides its
   built-in fallback form when it detects a shortcode.

**Out-of-the-box (no plugin):** the page already has a working form that
validates input, accepts a JPG/PNG/PDF upload, and emails the submission to your
site admin address. For reliable delivery install an SMTP plugin (below).

**Email deliverability:** install **WP Mail SMTP** and connect it to your email
provider so notifications don't land in spam. Set the lead recipient under
**Settings → General → Administration Email Address**, or have the developer
point `allied_landowner_recipient` at a dedicated inbox.

## 5. Gated portals (Builder / Investor / Partner)

**One-time setup (if not created by starter content):**
1. Create three pages: **Builder**, **Investor**, **Partner**, each with the
   **"Portal (Gated)"** template, and set each page's parent to **Portals** so
   the URLs become `/portals/builder/`, `/portals/investor/`, `/portals/partner/`.
   *The template gates by the page slug — keep slugs `builder`, `investor`,
   `partner`.*

**Giving someone access:**
1. **Users → Add New.** Create the account.
2. Set their **Role** to **Builder Member**, **Investor Member**, or **Partner
   Member** (installing the *Members* plugin gives a friendlier UI).
3. Send them their login. They sign in at the portal page (or `/wp-admin`) and
   will see the gated content; everyone else sees the branded sign-in screen.

**Adding documents to a portal:**
- Edit the portal page → add text/links in the body **and/or** attach PDFs to
  the page (upload via the editor). Attached files are listed automatically in
  the portal's "Documents" sidebar with their file type.

## 6. Branding, favicon & SEO basics

- **Logo:** Appearance → Customize → Site Identity → upload logo.
- **Favicon / Site Icon:** same panel → Site Icon (512×512 PNG).
- **Site title & tagline:** Settings → General (tagline shows in the footer).
- **Per-page SEO:** the theme outputs a meta description from each page's
  Excerpt plus Open Graph tags automatically. For finer control install Yoast or
  Rank Math.

## 7. Applying the approved homepage mockup (developer task)

All visual styling is centralized. To match the mockup precisely:
1. Edit `wp-content/themes/allied/assets/css/tokens.css` — replace the palette,
   font families, type scale, and spacing with the mockup values.
2. Add the licensed fonts to `assets/fonts/` and declare `@font-face` in
   `tokens.css` (keeps the build dependency-free — no Google Fonts CDN call).
3. Fine-tune section spacing/imagery on the homepage if the mockup differs.
   No template/structure changes should be required.

## 8. Where things live (for the developer)

```
wp-content/themes/allied/
├── assets/css/tokens.css      ← EDIT THIS to match the mockup
├── assets/css/{base,layout,components}.css
├── front-page.php             ← Home
├── archive-community.php      ← Portfolio grid
├── single-community.php       ← Individual project
├── page-templates/            ← Firm, Capabilities, Partners, Landowner, Contact, Portals, Portal
├── inc/                       ← setup, enqueue, CPT, portal roles, helpers, SEO
└── template-parts/            ← reusable card / CTA
```

---

## Loom walkthrough — suggested outline (5–7 min)

1. Logging in and the dashboard overview.
2. Editing a page (title, body, excerpt → banner).
3. Adding a Community with photos, details, region & status.
4. The landowner form and where lead emails go.
5. Creating a portal user and adding documents to a portal.
6. Updating the logo and favicon.
