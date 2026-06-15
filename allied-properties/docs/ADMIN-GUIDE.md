# ADMIN GUIDE — Allied Properties

For the non-technical site admin. How to run the site day to day.

## 1. Logging in
- `https://YOURDOMAIN/wp-admin` — use the admin credentials from handoff and
  change your password on first login (**Users → Profile**).

## 2. Pages & templates
Every page is under **Pages**. If you ever recreate one, assign the matching
**Template** in the right sidebar (Page → Template):

| Page                | Slug         | Template                |
|---------------------|--------------|-------------------------|
| Home                | `home`       | (Default — uses Home layout via Settings → Reading) |
| The Firm            | `the-firm`   | The Firm / About + Leadership |
| Capabilities        | `capabilities`| Capabilities           |
| Partners            | `partners`   | Partners                |
| Sell Us Your Land   | `landowners` | Landowner — Sell Us Your Land |
| Contact             | `contact`    | Contact                 |
| Portals             | `portals`    | Portals Hub             |
| Builder (child of Portals)  | `builder`  | Portal (Gated)    |
| Investor (child of Portals) | `investor` | Portal (Gated)    |
| Partner (child of Portals)  | `partner`  | Portal (Gated)    |

> The **Communities/Portfolio** grid is automatic at `/communities/` — it is not
> a Page, it's driven by the Community items you publish (see §3).

**Editing rules of thumb**
- The big banner uses the page **Title**.
- The intro line under it uses the page **Excerpt** (open the Excerpt panel from
  the three-dot menu → Preferences → Panels if you don't see it).
- The main body uses the normal block editor.

## 3. Communities (the portfolio)
1. **Communities → Add Community.**
2. Title + description (body) + **Featured image** (card & hero photo).
3. Fill the **Project Details** box: location, acreage, lots, builder, year.
4. Set a **Region** and a **Status** (e.g. *In Development*, *Delivered*) — these
   create the filter buttons on the Communities page.
5. Attach extra photos to the item to build the project gallery.
6. **Publish.** It appears in the grid and the homepage "Selected communities".

## 4. Forms & lead emails
**Recommended (Contact Form 7):**
1. Install/activate **Contact Form 7**.
2. **Contact → Add New.** Fields: Name*, Email*, Phone, County/City, Parcel
   ID/Address, Acreage, Message, and a file upload:
   `[file parcel-file limit:10mb filetypes:jpg|jpeg|png|webp|pdf]`
3. **Mail** tab → recipient = your acquisitions inbox; add `[parcel-file]` under
   *File attachments*.
4. Copy the shortcode → paste into the **Sell Us Your Land** page (and a separate
   form on **Contact**). The Landowner page auto-hides its built-in form when it
   detects a shortcode.

**No-plugin fallback:** the Landowner page already has a working form that
validates, accepts a JPG/PNG/PDF upload, and emails the admin.

**Deliverability:** install **WP Mail SMTP** so emails don't go to spam. Lead
recipient = **Settings → General → Administration Email Address** (or have the
developer point the `allied_landowner_recipient` filter at a dedicated inbox).

## 5. Portals (Builder / Investor / Partner)
**Give someone access**
1. **Users → Add New.**
2. Set **Role** to *Builder Member*, *Investor Member*, or *Partner Member*
   (the **Members** plugin gives a friendlier UI).
3. Send them their login. They sign in at the portal page; everyone else sees the
   branded sign-in screen.

**Add documents to a portal**
- Edit the portal page → add text/links in the body, and/or **attach PDFs** to
  the page. Attached files list automatically in the portal's Documents sidebar.

## 6. Branding, favicon, SEO
- **Logo & favicon:** Appearance → Customize → Site Identity (Site Icon = 512×512).
- **Titles/tagline:** Settings → General (tagline shows in the footer).
- **Per-page SEO:** the theme auto-generates a meta description from each page's
  Excerpt + Open Graph tags. Write a good Excerpt for each page. Install Yoast/
  Rank Math only if you want finer control.

## 7. Common gotchas
- **A community page shows 404:** Settings → Permalinks → Save (re-flush).
- **Form emails not arriving:** configure WP Mail SMTP; check spam.
- **Portal user sees "no access":** they have the wrong role for that portal.

## Loom walkthrough — suggested outline (5–7 min)
1. Login + dashboard. 2. Editing a page (title/excerpt/body). 3. Adding a
Community. 4. Lead form + where emails go. 5. Creating a portal user + adding
docs. 6. Logo + favicon.
