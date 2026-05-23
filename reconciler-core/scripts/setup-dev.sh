#!/usr/bin/env bash
# =============================================================
# setup-dev.sh – One-command Reconciler dev environment setup
# Usage: ./scripts/setup-dev.sh
# =============================================================

set -euo pipefail

# ── Colours ───────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
RESET='\033[0m'

log()  { echo -e "${BLUE}[setup]${RESET} $*"; }
ok()   { echo -e "${GREEN}[✓]${RESET} $*"; }
warn() { echo -e "${YELLOW}[!]${RESET} $*"; }
fail() { echo -e "${RED}[✗]${RESET} $*"; exit 1; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo ""
echo -e "${BOLD}╔══════════════════════════════════════════════╗${RESET}"
echo -e "${BOLD}║     Reconciler – Dev Environment Setup       ║${RESET}"
echo -e "${BOLD}╚══════════════════════════════════════════════╝${RESET}"
echo ""

# ─────────────────────────────────────────────────────────────
# 1. Check required dependencies
# ─────────────────────────────────────────────────────────────
log "Step 1/7 – Checking dependencies..."

check_cmd() {
  local cmd="$1"
  local install_hint="${2:-}"
  if ! command -v "$cmd" &>/dev/null; then
    fail "$cmd is not installed. ${install_hint}"
  fi
  ok "$cmd found: $(command -v "$cmd")"
}

check_cmd docker       "Install Docker: https://docs.docker.com/get-docker/"
check_cmd "docker" # also check compose plugin
if docker compose version &>/dev/null 2>&1; then
  ok "docker compose (plugin) found"
elif command -v docker-compose &>/dev/null; then
  ok "docker-compose found"
  # Alias for the rest of the script
  docker_compose() { docker-compose "$@"; }
else
  fail "Neither 'docker compose' plugin nor 'docker-compose' found."
fi

check_cmd rustc "Install Rust: https://rustup.rs/"
check_cmd cargo "Install Rust: https://rustup.rs/"
check_cmd curl  "Install curl via your package manager"

RUST_VERSION=$(rustc --version | awk '{print $2}')
REQUIRED_MAJOR=1
REQUIRED_MINOR=77
RUST_MAJOR=$(echo "$RUST_VERSION" | cut -d. -f1)
RUST_MINOR=$(echo "$RUST_VERSION" | cut -d. -f2)
if [ "$RUST_MAJOR" -lt "$REQUIRED_MAJOR" ] || \
   ([ "$RUST_MAJOR" -eq "$REQUIRED_MAJOR" ] && [ "$RUST_MINOR" -lt "$REQUIRED_MINOR" ]); then
  fail "Rust >= 1.77 required, found $RUST_VERSION. Run: rustup update"
fi
ok "Rust $RUST_VERSION OK"

echo ""

# ─────────────────────────────────────────────────────────────
# 2. Set up .env
# ─────────────────────────────────────────────────────────────
log "Step 2/7 – Setting up environment file..."

cd "$ROOT_DIR"

if [ -f ".env" ]; then
  warn ".env already exists — skipping copy (delete it to reset)"
else
  cp .env.example .env
  ok "Copied .env.example → .env"
  warn "Edit .env and fill in CLERK_SECRET_KEY, CLERK_JWT_KEY, etc. before running the app"
fi

echo ""

# ─────────────────────────────────────────────────────────────
# 3. Start infrastructure services
# ─────────────────────────────────────────────────────────────
log "Step 3/7 – Starting Postgres, NATS, Redis..."

docker compose up -d postgres nats redis 2>&1 | sed 's/^/  /'

ok "Infrastructure containers started"
echo ""

# ─────────────────────────────────────────────────────────────
# 4. Wait for Postgres to be ready
# ─────────────────────────────────────────────────────────────
log "Step 4/7 – Waiting for Postgres to be ready..."

RETRIES=30
for i in $(seq 1 "$RETRIES"); do
  if docker compose exec -T postgres \
       pg_isready -U "${POSTGRES_USER:-reconciler}" -d "${POSTGRES_DB:-reconciler}" \
       &>/dev/null; then
    ok "Postgres is ready (attempt $i/$RETRIES)"
    break
  fi
  if [ "$i" -eq "$RETRIES" ]; then
    fail "Postgres did not become ready after $RETRIES attempts"
  fi
  echo -n "."
  sleep 2
done

echo ""

# ─────────────────────────────────────────────────────────────
# 5. Run database migrations
# ─────────────────────────────────────────────────────────────
log "Step 5/7 – Running database migrations..."

# Load .env for DATABASE_URL
set -a; source .env; set +a

if cargo build --bin migrate --quiet 2>&1; then
  cargo run --bin migrate --quiet
  ok "Migrations applied"
else
  warn "No 'migrate' binary found — skipping (add src/bin/migrate.rs to automate this)"
fi

echo ""

# ─────────────────────────────────────────────────────────────
# 6. Seed test data
# ─────────────────────────────────────────────────────────────
log "Step 6/7 – Seeding development test data..."

if [ -f "scripts/seed.sql" ]; then
  docker compose exec -T postgres \
    psql -U "${POSTGRES_USER:-reconciler}" -d "${POSTGRES_DB:-reconciler}" \
    < scripts/seed.sql
  ok "Test data seeded from scripts/seed.sql"
elif cargo build --bin seed --quiet 2>&1; then
  cargo run --bin seed --quiet
  ok "Test data seeded via 'seed' binary"
else
  warn "No seed script found — skipping test data"
fi

echo ""

# ─────────────────────────────────────────────────────────────
# 7. Start the API
# ─────────────────────────────────────────────────────────────
log "Step 7/7 – Starting Reconciler API..."

echo ""
echo -e "${BOLD}╔══════════════════════════════════════════════╗${RESET}"
echo -e "${BOLD}║  Reconciler dev environment ready! 🚀        ║${RESET}"
echo -e "${BOLD}╠══════════════════════════════════════════════╣${RESET}"
echo -e "${BOLD}║  API:      ${GREEN}http://localhost:8080${RESET}${BOLD}             ║${RESET}"
echo -e "${BOLD}║  Sandbox:  ${GREEN}http://localhost:8081${RESET}${BOLD}             ║${RESET}"
echo -e "${BOLD}║  Adminer:  ${GREEN}http://localhost:8082${RESET}${BOLD}             ║${RESET}"
echo -e "${BOLD}║  NATS Mon: ${GREEN}http://localhost:8083${RESET}${BOLD}             ║${RESET}"
echo -e "${BOLD}╚══════════════════════════════════════════════╝${RESET}"
echo ""
echo -e "  ${YELLOW}Tip:${RESET} Run ${BOLD}cargo run${RESET} to start in development mode"
echo -e "  ${YELLOW}Tip:${RESET} Run ${BOLD}docker compose logs -f${RESET} to tail all logs"
echo -e "  ${YELLOW}Tip:${RESET} Run ${BOLD}docker compose down -v${RESET} to reset everything"
echo ""

# Auto-start if --run flag is passed
if [[ "${1:-}" == "--run" ]]; then
  log "Starting API with cargo run..."
  cargo run
fi
