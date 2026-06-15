# Local Preview — one command

Run the Allied Properties site on your own machine and open it in a browser.

## Quick start (no MySQL, no Docker)

```bash
cd allied-properties/preview
./start.sh
```

Then open **http://localhost:8088**

- **Site:** http://localhost:8088/
- **Admin:** http://localhost:8088/wp-admin/ — `admin` / `Allied!Preview123`
- **Portal demo:** http://localhost:8088/portals/builder/ — `builder1` / `Builder!Preview123`

What `start.sh` does (idempotent — safe to re-run):
1. Downloads WordPress core (WordPress.org, with a GitHub mirror fallback).
2. Installs the SQLite database integration (so no MySQL is needed).
3. Writes `wp-config.php`, installs WordPress, activates the **Allied** theme.
4. Seeds all pages with the right templates, 2 sample communities, the primary
   menu, a portal user, sets the static homepage + pretty permalinks.
5. Starts a PHP server at `http://localhost:8088`.

**Requirements:** PHP 8.0+ (`pdo_sqlite`, `gd`), plus `curl`/`wget` and `unzip`.

Useful flags:
```bash
PORT=9000 ./start.sh     # different port
./start.sh --reset       # wipe runtime and rebuild
./start.sh --setup-only  # build + seed without serving (CI)
```

The runtime lives in `preview/.runtime/` (git-ignored). Delete it any time.

## Alternative: Docker (MySQL-backed)

From `allied-properties/`:

```bash
docker compose up -d        # http://localhost:8088
```

Then activate the theme in `wp-admin` and run the same setup steps from
`docs/INSTALL.md` (or copy `preview/seed.php` into the container and run it).

> Note: a preview running on *this* repo's cloud build environment is not
> reachable from your browser — run `start.sh` on your own machine to get a URL
> you can open.
