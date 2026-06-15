# START — run the Allied Properties preview locally

A reproducible local copy you can open in a browser. No MySQL, no Docker.

## Prerequisites
- **PHP 8.0+** with the `pdo_sqlite` and `gd` extensions
  - check: `php -v` and `php -m | grep -E 'pdo_sqlite|gd'`
- **curl** or **wget**, and **unzip**
- Internet access on first run (downloads WordPress core once)

## Install
```bash
git clone <your-repo-url>
cd <repo>/allied-properties/preview
```
No build step — `start.sh` fetches everything it needs on first run.

## Start
```bash
./start.sh
```
Then open **http://localhost:8088**

| Where | URL | Credentials |
|---|---|---|
| Site | http://localhost:8088/ | — |
| Admin | http://localhost:8088/wp-admin/ | `admin` / `Allied!Preview123` |
| Portal demo | http://localhost:8088/portals/builder/ | `builder1` / `Builder!Preview123` |

> Credentials are defined in `preview/install.php` (admin) and `preview/seed.php`
> (portal user). Override the admin with env vars:
> `ALLIED_ADMIN_USER`, `ALLIED_ADMIN_PASS`, `ALLIED_ADMIN_EMAIL`.

Use a different port:
```bash
PORT=9000 ./start.sh      # http://localhost:9000
```

## Stop
Press **Ctrl + C** in the terminal running `start.sh`.

## Reset (wipe everything and rebuild)
```bash
./start.sh --reset        # deletes preview/.runtime, rebuilds, serves
# or just remove the runtime:
rm -rf preview/.runtime
```
Re-running `./start.sh` after a reset re-downloads core, reinstalls, reseeds.

## Troubleshooting
| Symptom | Fix |
|---|---|
| `pdo_sqlite is required` | Enable the SQLite extension in `php.ini` (`extension=pdo_sqlite`). |
| `need curl or wget` / download fails | Install curl; check internet/proxy. `start.sh` falls back to a GitHub mirror automatically if WordPress.org is blocked. |
| Port already in use | `PORT=9000 ./start.sh`, or stop the other process. |
| A page shows 404 | Already handled by the seed (pretty permalinks flushed). If you imported manually: WP Admin → Settings → Permalinks → Save. |
| CSS/JS not loading | Hard-refresh; confirm you opened the `http://localhost:PORT` URL printed by the script. |
| File upload returns 404 locally | Known quirk of PHP's built-in dev server with multipart POST; works on real Apache/nginx. Use a CF7/WPForms plugin form for production. |
| Windows | Use Git Bash or WSL to run `./start.sh` (it's a bash script). See `LOCAL_SETUP.md`. |

## What's running
- WordPress core in `preview/.runtime/web/` (git-ignored, disposable)
- SQLite database at `preview/.runtime/web/wp-content/database/.ht.sqlite`
- The `allied` theme is synced from `wp-content/themes/allied/` on every start,
  so edits to the theme show up after a refresh / restart.
