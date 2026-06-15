#!/usr/bin/env bash
#
# Allied Properties — one-command local preview.
#
#   ./start.sh                 # boot + serve at http://localhost:8088
#   PORT=9000 ./start.sh       # use a different port
#   ./start.sh --setup-only    # build/seed without starting the server (CI)
#   ./start.sh --reset         # wipe the runtime and rebuild from scratch
#
# Requirements: php (8.0+ with pdo_sqlite, gd), curl OR wget, unzip.
# No MySQL, no Docker, no external WordPress.org account needed.
#
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
THEME_SRC="$(cd "$DIR/.." && pwd)/wp-content/themes/allied"
RUNTIME="$DIR/.runtime"
WPROOT="$RUNTIME/web"
PORT="${PORT:-8088}"
WP_VERSION="6.5"   # used for the GitHub fallback branch

# WordPress.org first; GitHub mirror as fallback (works behind some firewalls).
WP_ORG_URL="https://wordpress.org/wordpress-${WP_VERSION}.zip"
WP_GH_URL="https://codeload.github.com/WordPress/WordPress/zip/refs/heads/${WP_VERSION}-branch"
SQLITE_GH_URL="https://codeload.github.com/WordPress/sqlite-database-integration/zip/refs/heads/main"

SETUP_ONLY=0
for arg in "$@"; do
  case "$arg" in
    --setup-only) SETUP_ONLY=1 ;;
    --reset) echo "Resetting runtime…"; rm -rf "$RUNTIME" ;;
  esac
done

log() { printf "\033[1;34m›\033[0m %s\n" "$*"; }
die() { printf "\033[1;31m✗ %s\033[0m\n" "$*" >&2; exit 1; }

command -v php >/dev/null || die "php is required (8.0+)."
php -m | grep -qi pdo_sqlite || die "PHP extension pdo_sqlite is required."
fetch() { # fetch URL OUTFILE
  if command -v curl >/dev/null; then curl -fsSL -m 300 -o "$2" "$1";
  elif command -v wget >/dev/null; then wget -q -T 300 -O "$2" "$1";
  else die "need curl or wget"; fi
}

mkdir -p "$RUNTIME"

# ---- 1. WordPress core ------------------------------------------------------
if [ ! -f "$WPROOT/wp-load.php" ]; then
  log "Downloading WordPress core…"
  if ! fetch "$WP_ORG_URL" "$RUNTIME/wp.zip" 2>/dev/null; then
    log "WordPress.org unreachable, using GitHub mirror…"
    fetch "$WP_GH_URL" "$RUNTIME/wp.zip"
  fi
  log "Extracting…"
  ( cd "$RUNTIME" && unzip -q -o wp.zip )
  # The org zip extracts to wordpress/, the GitHub zip to WordPress-<branch>/
  SRC="$(find "$RUNTIME" -maxdepth 1 -type d \( -iname 'wordpress' -o -iname 'WordPress-*' \) | head -1)"
  [ -n "$SRC" ] || die "could not locate extracted WordPress"
  rm -rf "$WPROOT"; mv "$SRC" "$WPROOT"
  rm -f "$RUNTIME/wp.zip"
else
  log "WordPress core present."
fi

# ---- 2. SQLite drop-in ------------------------------------------------------
if [ ! -f "$WPROOT/wp-content/db.php" ]; then
  log "Installing SQLite database integration…"
  fetch "$SQLITE_GH_URL" "$RUNTIME/sqlite.zip"
  ( cd "$RUNTIME" && unzip -q -o sqlite.zip )
  rm -rf "$WPROOT/wp-content/plugins/sqlite-database-integration"
  mv "$RUNTIME/sqlite-database-integration-main" "$WPROOT/wp-content/plugins/sqlite-database-integration"
  cp "$WPROOT/wp-content/plugins/sqlite-database-integration/db.copy" "$WPROOT/wp-content/db.php"
  sed -i.bak "s#{SQLITE_IMPLEMENTATION_FOLDER_PATH}#$WPROOT/wp-content/plugins/sqlite-database-integration#g" "$WPROOT/wp-content/db.php"
  sed -i.bak "s#{SQLITE_PLUGIN}#sqlite-database-integration/load.php#g" "$WPROOT/wp-content/db.php"
  rm -f "$WPROOT/wp-content/db.php.bak" "$RUNTIME/sqlite.zip"
  mkdir -p "$WPROOT/wp-content/database"
fi

# ---- 3. wp-config.php -------------------------------------------------------
if [ ! -f "$WPROOT/wp-config.php" ]; then
  log "Writing wp-config.php…"
  cat > "$WPROOT/wp-config.php" <<PHP
<?php
define('DB_NAME','wordpress'); define('DB_USER','root'); define('DB_PASSWORD','');
define('DB_HOST','localhost'); define('DB_CHARSET','utf8'); define('DB_COLLATE','');
define('AUTH_KEY','k1');define('SECURE_AUTH_KEY','k2');define('LOGGED_IN_KEY','k3');define('NONCE_KEY','k4');
define('AUTH_SALT','s1');define('SECURE_AUTH_SALT','s2');define('LOGGED_IN_SALT','s3');define('NONCE_SALT','s4');
\$table_prefix='wp_';
define('WP_DEBUG', true); define('WP_DEBUG_LOG', true); define('WP_DEBUG_DISPLAY', false);
define('WP_HOME','http://localhost:${PORT}'); define('WP_SITEURL','http://localhost:${PORT}');
if (!defined('ABSPATH')) define('ABSPATH', __DIR__ . '/');
require_once ABSPATH . 'wp-settings.php';
PHP
fi

# ---- 4. Sync the theme (always — picks up latest code) ----------------------
log "Syncing Allied theme…"
mkdir -p "$WPROOT/wp-content/themes/allied"
cp -R "$THEME_SRC/." "$WPROOT/wp-content/themes/allied/"

# ---- 5. Install + seed (idempotent) ----------------------------------------
log "Installing WordPress (idempotent)…"
php "$DIR/install.php" "$WPROOT"
log "Seeding content (idempotent)…"
php "$DIR/seed.php" "$WPROOT" | sed 's/^/   /'

# ---- 6. Serve ---------------------------------------------------------------
URL="http://localhost:${PORT}"
echo
echo "──────────────────────────────────────────────"
echo "  Allied Properties preview is ready."
echo "  Site:   ${URL}/"
echo "  Admin:  ${URL}/wp-admin/   (admin / Allied!Preview123)"
echo "  Portal: ${URL}/portals/builder/  (builder1 / Builder!Preview123)"
echo "──────────────────────────────────────────────"

if [ "$SETUP_ONLY" -eq 1 ]; then
  log "--setup-only: not starting server."
  exit 0
fi

log "Starting PHP server on 0.0.0.0:${PORT} (Ctrl+C to stop)…"
export ALLIED_WP_ROOT="$WPROOT"
exec php -d display_errors=0 -d upload_max_filesize=16M -d post_max_size=16M \
  -S 0.0.0.0:"${PORT}" "$DIR/router.php"
