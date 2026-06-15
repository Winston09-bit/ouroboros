# MOCKUP INTEGRATION PLAN — Allied Properties

How the approved homepage mockup becomes the live design. The build was
structured so this is a **token swap + font install**, not a rebuild.

## What I need from you
1. The mockup file (Figma link, PDF, or high-res PNG of the homepage).
2. The **exact palette** (hex values): primary, accent, ink/text, surfaces,
   borders, on-dark text.
3. **Typography:** font families for headings + body, the licensed font files
   (woff2 preferred) or the foundry/source, and the intended size scale.
4. Any spacing/grid specifics visible in the mockup (section rhythm, container
   width, button styling, corner radius).

## Where everything maps (single source of truth)
All visual decisions live in **`assets/css/tokens.css`** as CSS custom
properties. The rest of the CSS references these tokens, so changing them
re-skins the whole site.

| Mockup element            | Token(s) to set in `tokens.css`                     |
|---------------------------|------------------------------------------------------|
| Primary / navy            | `--color-primary`, `--color-primary-700`             |
| Accent (brass/gold/etc.)  | `--color-accent`, `--color-accent-600`               |
| Text / headings           | `--color-ink`                                         |
| Section bands / paper      | `--color-surface`, `--color-surface-2`              |
| Hairlines/borders         | `--color-line`                                       |
| On-dark text              | `--color-on-dark`, `--color-on-dark-mut`             |
| Heading font              | `--font-display`                                      |
| Body font                 | `--font-body`                                         |
| Type scale                | `--fs-*` (clamp values)                               |
| Spacing rhythm            | `--space-*`                                           |
| Container width           | `--container`, `--container-narrow`                   |
| Corner radius             | `--radius`, `--radius-lg`                             |

The block editor palette/fonts in **`theme.json`** should be updated to match
the same values so the admin editing experience is on-brand.

## Steps (developer, ~0.5–1 day depending on mockup fidelity)
1. Add font files to `assets/fonts/` and declare `@font-face` at the top of
   `tokens.css`; point `--font-display` / `--font-body` at them. (Self-hosted →
   honors the no-external-dependency rule; no Google Fonts CDN call.)
2. Replace the placeholder color + scale tokens with the mockup values.
3. Mirror the palette/fonts into `theme.json`.
4. Drop in the supplied hero/section imagery; set the Home featured image.
5. Pixel-compare Home against the mockup; nudge section padding/hero overlay/
   button sizing only via tokens/section utility classes.
6. Re-run the QA crawl (`docs/DEPLOYMENT-CHECKLIST.md` → Quality gates).

## What will NOT change
- Page templates, routes, custom post type, taxonomies, forms, portals, and
  information architecture are final and do not need rework to match the mockup.

## Risk notes
- If the mockup introduces components not in the current system (e.g. an
  interactive map, a stat carousel), those are additive and will be scoped
  separately — they don't block the re-skin of existing pages.
