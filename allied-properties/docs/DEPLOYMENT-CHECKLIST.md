# DEPLOYMENT CHECKLIST — Allied Properties

Work top to bottom before announcing the site. Tick each item.

## Environment
- [ ] WordPress 6.4+ / PHP 8.0+ confirmed on host
- [ ] HTTPS enabled and forced (http → https redirect)
- [ ] `WP_DEBUG` **off** in production `wp-config.php`
- [ ] Site/Home URL set to the production domain
- [ ] Automated backups configured (files + DB)

## Theme & structure
- [ ] `allied` theme activated
- [ ] All pages exist with correct templates (Home, The Firm, Capabilities,
      Communities page note*, Partners, Landowners, Contact, Portals + 3 portals)
- [ ] Static homepage set (Settings → Reading)
- [ ] Permalinks set to "Post name" and saved
- [ ] Primary + footer menus assigned (Appearance → Menus)

\*The portfolio lives at the CPT archive `/communities/` (no page needed).

## Content
- [ ] Real homepage hero image uploaded (Featured image on Home, or supplied art)
- [ ] At least 3 Communities published with photo + details + region + status
- [ ] Leadership team replaced (The Firm)
- [ ] Partner logos uploaded (Partners / homepage logo wall)
- [ ] Contact details + email correct (footer + Contact page)

## Forms (lead capture)
- [ ] SMTP plugin configured and a **test lead email received**
- [ ] Landowner form submits + file upload works + email arrives
- [ ] Contact form submits + email arrives
- [ ] Lead recipient address is correct (admin email or `allied_landowner_recipient`)

## Portals
- [ ] Builder / Investor / Partner pages use the "Portal (Gated)" template
- [ ] Portal slugs are exactly `builder`, `investor`, `partner`
- [ ] One test user per role created; each sees only its own portal
- [ ] Sample document attached to a portal page and visible to that role
- [ ] Logged-out users see the branded sign-in screen (not a blank page)

## SEO / metadata
- [ ] Site title + tagline set
- [ ] Favicon / Site Icon uploaded
- [ ] Each page has a meaningful Excerpt (drives meta description)
- [ ] `robots`/search-engine visibility ON (Settings → Reading — uncheck
      "Discourage search engines" when ready to launch)
- [ ] Open Graph image shows when sharing a community (has featured image)

## Quality gates
- [ ] All nav links resolve (no 404s)
- [ ] Mobile menu opens/closes; layout holds at 360px width
- [ ] No PHP warnings in `wp-content/debug.log` during a full crawl
- [ ] 404 page renders branded content
- [ ] Images have alt text (set in Media library)

## Handover
- [ ] Admin account handed to client; client changed password
- [ ] `docs/ADMIN-GUIDE.md` shared
- [ ] Loom walkthrough recorded (outline in ADMIN-GUIDE)
