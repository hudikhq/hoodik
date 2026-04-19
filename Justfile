# Default: list available recipes
default:
    @just --list

# ── Development ───────────────────────────────────────────────────────────────

# Run frontend (Vite) and backend (cargo-watch) together
# Rebuilds WASM first so the browser always gets the latest crypto code.
# Kills any stale hoodik binaries left over from previous sessions before starting.
dev: wasm build-editor
    #!/usr/bin/env bash
    set -euo pipefail
    trap 'kill 0' EXIT

    # Kill any stale hoodik processes (debug or release) from previous sessions.
    stale=$(pgrep -f "target/(debug|release)/hoodik" 2>/dev/null || true)
    if [ -n "$stale" ]; then
        echo "⚠  Killing stale hoodik process(es): $stale"
        kill $stale 2>/dev/null || true
        sleep 0.5
    fi

    echo "Starting Vite dev server..."
    yarn workspace @hoodik/web run dev &

    echo "Starting Rust backend with auto-reload..."
    # hoodik/build.rs regenerates hoodik/src/client.rs on every build; that touches a .rs file and
    # would otherwise restart cargo-watch in an infinite loop. Ignore it + churn from the web bundle.
    cargo watch \
        --watch-when-idle \
        -i "hoodik/src/client.rs" \
        -i "web/**" \
        -i "**/node_modules/**" \
        -i "transfer/pkg/**" \
        -x run &

    wait

# Run only the Vite frontend dev server
dev-web:
    yarn workspace @hoodik/web run dev

# Run only the Rust backend with auto-reload on save
dev-api:
    cargo watch \
        --watch-when-idle \
        -i "hoodik/src/client.rs" \
        -i "web/**" \
        -i "**/node_modules/**" \
        -i "transfer/pkg/**" \
        -x run

# Run the Rust backend once without auto-reload
run:
    cargo run

# ── WASM ──────────────────────────────────────────────────────────────────────

# Build the transfer WASM crate (includes all cryptfns exports)
wasm:
    yarn workspace @hoodik/transfer run wasm-pack
    mkdir -p web/node_modules/transfer
    cp -R transfer/pkg/. web/node_modules/transfer/

# ── Editor ────────────────────────────────────────────────────────────────────

# Build the @hoodik/editor workspace. web/ imports the compiled bundle from
# editor/dist/, which is gitignored — a fresh checkout (or CI) has no dist
# yet, so build-web (and test-web, dev, e2e) must run this first.
build-editor:
    yarn workspace @hoodik/editor run build

# ── Testing ───────────────────────────────────────────────────────────────────

# Run all tests (Rust + frontend unit)
test: test-rust test-web

# Run all Rust tests (unit + integration)
test-rust: test-rust-unit test-rust-integration

# Run Rust unit tests across the workspace
test-rust-unit:
    cargo test --workspace --lib -- --nocapture

# Run Rust integration tests (auth, storage, links, email)
test-rust-integration:
    cargo test --test web_authentication -- --nocapture
    cargo test --test storage -- --nocapture
    cargo test --test storage_replace_content -- --nocapture
    cargo test --test storage_set_editable -- --nocapture
    cargo test --test storage_legacy_routing -- --nocapture
    cargo test --test links -- --nocapture
    cargo test --test email -- --nocapture

# Run Rust tests for the transfer crate only
test-transfer:
    cargo test -p transfer

# Run frontend unit tests (Vitest)
test-web: build-editor
    #!/usr/bin/env bash
    set -euo pipefail
    # transfer WASM uses externref; older Node versions can't parse it and
    # every spec fails to load with a cryptic WebAssembly.compile() error.
    # Fail loud with the real fix instead of letting the user chase it.
    required=$(cat .nvmrc 2>/dev/null || echo 22)
    actual=$(node --version 2>/dev/null | sed 's/^v\([0-9]*\).*/\1/')
    if [ -z "$actual" ] || [ "$actual" -lt "$required" ]; then
        echo "error: Node ${required}+ required (got $(node --version 2>/dev/null || echo 'none'))"
        echo "       The transfer WASM crate uses reference-types (externref) that older Node rejects."
        echo "       Run 'nvm use' (reads .nvmrc) or install Node ${required}."
        exit 1
    fi
    yarn workspace @hoodik/web run test:unit

# Run frontend unit tests in watch mode
test-watch:
    yarn workspace @hoodik/web run test:watch

# Run E2E tests: build backend, start it, run Playwright, then clean up
e2e:
    #!/usr/bin/env bash
    set -eo pipefail

    export DATA_DIR=$PWD/data-e2e
    export ENV_FILE=".env.e2e"

    # Force plain HTTP for the e2e server regardless of what .env.e2e says,
    # and override the base URL so Playwright connects on http://.
    # MAILER_TYPE=none auto-verifies new accounts (no email confirmation needed).
    export SSL_DISABLED=true
    export APP_URL=http://localhost:5443
    export APP_CLIENT_URL=http://localhost:5443
    export MAILER_TYPE=none

    mkdir -p "$DATA_DIR"

    cargo build --bin hoodik --release

    RUST_LOG=error $PWD/target/release/hoodik &
    SERVER_PID=$!

    cleanup() { kill -9 $SERVER_PID 2>/dev/null; rm -rf $PWD/data-e2e; }
    trap cleanup EXIT

    node_modules/.bin/wait-on -t 600000 http://127.0.0.1:5443/api/liveness

    export ENV_FILE="../.env.e2e"
    yarn workspace @hoodik/web test:e2e

# Open Playwright test UI interactively (useful for debugging)
e2e-ui:
    yarn workspace @hoodik/web test:e2e:ui

# ── Code Quality ──────────────────────────────────────────────────────────────

# Run Clippy with warnings as errors
clippy:
    cargo clippy -- -D warnings

# Run Cargo check across the workspace
check-rust:
    cargo check

# Run TypeScript type checking
check-types:
    yarn workspace @hoodik/web run type-check

# Run all checks (clippy + TypeScript)
check: clippy check-types

# Run ESLint on the web frontend
lint-web:
    yarn workspace @hoodik/web run lint

# Run all linters (clippy + ESLint)
lint: clippy lint-web

# ── Build ─────────────────────────────────────────────────────────────────────

# Build the web frontend for production
build-web: build-editor
    yarn workspace @hoodik/web run build

# Build the Rust backend in release mode
build-api:
    cargo build --release

# Build everything (WASM + web + API)
build: wasm build-web build-api

# ── Database ──────────────────────────────────────────────────────────────────

# Start the PostgreSQL container
db-up:
    docker compose up -d postgres

# Stop the PostgreSQL container
db-down:
    docker compose stop postgres

# ── MinIO (S3) ───────────────────────────────────────────────────────────────

# Start MinIO and create the default bucket
minio-up:
    docker compose up -d minio minio-init

# Stop MinIO
minio-down:
    docker compose stop minio

# Migrate file chunks from local filesystem to S3 (requires S3 env vars)
migrate-storage:
    cargo run --release -- migrate-storage

# Run the S3 versioned-chunk integration suite.
# Loads `.env` if present (see `.env.example` for the S3_* variables);
# otherwise falls back to MinIO defaults and brings the container up.
# Works against any S3-compatible backend — AWS, MinIO, Cloudflare R2, etc.
test-s3-integration:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -f .env ]; then
        echo "Loading .env for S3 integration tests..."
        set -a; source ./.env; set +a
    fi
    endpoint="${S3_ENDPOINT:-http://127.0.0.1:9000}"
    case "$endpoint" in
        *127.0.0.1*|*localhost*)
            echo "Using MinIO at $endpoint — ensuring container is up..."
            just minio-up
            ;;
        *)
            echo "Using remote S3 at $endpoint — skipping MinIO startup."
            ;;
    esac
    cargo test -p fs --features s3-integration-tests providers::s3::s3_versioned_tests -- --nocapture

# ── Setup ─────────────────────────────────────────────────────────────────────

# First-time setup: install deps, build WASM, copy env
setup:
    #!/usr/bin/env bash
    set -euo pipefail

    echo "Installing JS dependencies..."
    yarn install

    if [ ! -f .env ]; then
        echo "Creating .env from .env.example..."
        cp .env.example .env
    fi

    echo "Building WASM crates..."
    just wasm

    echo "Building @hoodik/editor..."
    just build-editor

    echo "Installing Playwright browsers..."
    cd web && npx playwright install chromium

    echo "Done! Run 'just dev' to start developing."

# ── CI Helpers ────────────────────────────────────────────────────────────────

# Full CI test pipeline (used by GitHub Actions)
ci-test: clippy test-rust-unit test-rust-integration wasm test-web build-web e2e
