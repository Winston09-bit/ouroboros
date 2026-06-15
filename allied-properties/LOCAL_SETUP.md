# LOCAL_SETUP — download and run on your own machine

Two ways to get the project: **(A)** the downloadable ZIP, or **(B)** git.
Then follow the Mac or Windows steps. Exact commands only.

## Get the project
- **A — ZIP:** download `allied-properties.zip` (sent in chat), unzip it.
- **B — Git:**
  ```
  git clone <your-repo-url>
  ```

---

## A. macOS

```bash
# 1. Prerequisites (Homebrew)
brew install php
php -m | grep -E 'pdo_sqlite|gd'      # both should print

# 2. Go to the preview folder
cd allied-properties/preview          # (or cd <unzipped>/allied-properties/preview)

# 3. Start
./start.sh

# 4. Open in your browser
open http://localhost:8088
```
Stop: `Ctrl + C`  •  Reset: `./start.sh --reset`

---

## B. Windows

PHP on Windows runs the same script via **Git Bash** (ships with Git for
Windows) or **WSL**. Pick one.

### Option 1 — Git Bash
```bash
# 1. Install PHP for Windows (https://windows.php.net/download) and add it to PATH.
#    In php.ini, ensure these are uncommented:
#       extension=pdo_sqlite
#       extension=gd
php -v
php -m | findstr sqlite

# 2. Open "Git Bash", go to the preview folder
cd allied-properties/preview

# 3. Start
./start.sh

# 4. Open http://localhost:8088 in your browser
```

### Option 2 — WSL (Ubuntu)
```bash
sudo apt update && sudo apt install -y php-cli php-sqlite3 php-gd unzip curl
cd allied-properties/preview
./start.sh
# open http://localhost:8088
```
Stop: `Ctrl + C`  •  Reset: `./start.sh --reset`

---

## One startup command
```
./start.sh
```

## Quick reference
```
DOWNLOAD:  allied-properties.zip   (or: git clone <your-repo-url>)
INSTALL:   cd allied-properties/preview
START:     ./start.sh
OPEN:      http://localhost:8088
LOGIN:     admin / Allied!Preview123     (portal: builder1 / Builder!Preview123)
STOP:      Ctrl + C
RESET:     ./start.sh --reset
```

## Don't want to run anything yet?
Open the standalone `*.html` snapshots included in the ZIP (`preview-snapshots/`)
directly in any browser — they render the pages offline with styling inlined.

## Notes
- First run downloads WordPress core once (cached in `preview/.runtime/`).
- No MySQL/Docker needed; the database is a single SQLite file.
- The cloud build environment cannot expose a browser URL to you — that's why
  you run `start.sh` locally to get `http://localhost:8088`.
