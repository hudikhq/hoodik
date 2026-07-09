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
    # Kill any stale hoodik processes (debug or release) from previous sessions.
    stale=$(pgrep -f "target/(debug|release)/hoodik" 2>/dev/null || true)
    if [ -n "$stale" ]; then
        echo "⚠  Killing stale hoodik process(es): $stale"
        kill $stale 2>/dev/null || true
        sleep 0.5
    fi

    # Vite (frontend, hot-reload) runs in the background on :5173; its output
    # goes to a log so bacon's terminal UI stays clean, and it's torn down on exit.
    yarn workspace @hoodik/web run dev >/tmp/hoodik-vite.log 2>&1 &
    vite_pid=$!
    trap 'kill $vite_pid 2>/dev/null' EXIT
    echo "▶ Vite (frontend)  http://localhost:5173   (logs: /tmp/hoodik-vite.log)"
    echo "▶ Backend in bacon below — rebuilds + reruns on save; press r to restart, q to quit."
    echo

    # bacon's run-long job builds and runs the server, restarting on source
    # changes, with interactive r=restart / q=quit. Replaces cargo-watch, which
    # hangs at startup when given this workspace's full -w watch set.
    bacon run-long

# Run only the Vite frontend dev server
dev-web:
    yarn workspace @hoodik/web run dev

# Run only the Rust backend (bacon: rebuild + rerun on save; r=restart, q=quit)
dev-api:
    bacon run-long

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

# Run Rust integration tests (auth, storage, links, email, shares fixtures)
test-rust-integration:
    cargo test --test web_authentication -- --nocapture
    cargo test --test web_liveness -- --nocapture
    cargo test --test readiness -- --nocapture
    cargo test --test web_registration -- --nocapture
    cargo test --test storage -- --nocapture
    cargo test --test storage_replace_content -- --nocapture
    cargo test --test storage_set_editable -- --nocapture
    cargo test --test storage_legacy_routing -- --nocapture
    cargo test --test storage_tar_upload -- --nocapture
    cargo test --test storage_instance_quota -- --nocapture
    cargo test --test links -- --nocapture
    cargo test --test email -- --nocapture
    cargo test --test shares_asn1_fixtures -- --nocapture
    cargo test --test shares_basic -- --nocapture
    cargo test --test shares_folder -- --nocapture
    cargo test --test shares_editable_folders -- --nocapture
    cargo test --test shares_permissions -- --nocapture
    cargo test --test shares_search -- --nocapture
    cargo test --test shares_audit -- --nocapture
    cargo test --test shares_admin_kill_switch -- --nocapture
    cargo test --test shares_default_cipher -- --nocapture
    cargo test --test shares_dual_key -- --nocapture
    cargo test --test key_transitions -- --nocapture
    cargo test --test opaque_login -- --nocapture
    cargo test --test migration -- --nocapture
    cargo test --test register_v2 -- --nocapture
    cargo test --test shares_groups -- --nocapture
    cargo test --test shares_fork -- --nocapture
    cargo test --test shares_quota -- --nocapture
    cargo test --test shares_discover -- --nocapture
    cargo test --test shares_account_deletion -- --nocapture
    cargo test --test shares_email -- --nocapture
    cargo test --test shares_recipient_navigation -- --nocapture

# ── Postgres integration testing ──────────────────────────────────────────────

# Start the throwaway Postgres container used by the integration suite
# and block until its healthcheck reports ready.
test-pg-up:
    #!/usr/bin/env bash
    set -euo pipefail
    docker compose up -d postgres-test
    echo "Waiting for postgres-test to be healthy..."
    for i in $(seq 1 60); do
        status=$(docker inspect -f '{{{{.State.Health.Status}}' postgres-test 2>/dev/null || echo "starting")
        if [ "$status" = "healthy" ]; then
            echo "postgres-test is healthy"
            exit 0
        fi
        sleep 1
    done
    echo "postgres-test did not become healthy within 60s"
    docker compose logs postgres-test
    exit 1

# Stop and remove the throwaway Postgres container.
test-pg-down:
    docker compose stop postgres-test
    docker compose rm -f postgres-test

# Run the integration suite against Postgres. Brings the container up
# first and tears it down on exit regardless of test outcome — the trap
# fires on success, failure, and Ctrl-C alike.
test-rust-integration-pg:
    #!/usr/bin/env bash
    set -euo pipefail
    just test-pg-up
    trap 'just test-pg-down' EXIT
    export TEST_DATABASE_URL="postgres://hoodik_test:hoodik_test@localhost:5433/hoodik_test"
    just test-rust-integration

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

# Run E2E tests: build backend, start it, run Playwright, then clean up.
# Pass any Playwright flags as extra args, e.g.
#   PWSLOWMO=500 just e2e --headed
#   just e2e --headed -g "Reader"
#   just e2e e2e/shares-basic.spec.ts --headed
#   just e2e --debug -g "Reader"
e2e *args:
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

    # The server bundles `web/dist/` into its binary via hoodik/build.rs,
    # so a stale or missing `dist` makes every Playwright `page.goto` land
    # on an empty 200 OK and every test time out waiting for selectors.
    just wasm
    just build-web

    cargo build --bin hoodik --release

    RUST_LOG=error $PWD/target/release/hoodik &
    SERVER_PID=$!

    cleanup() { kill -9 $SERVER_PID 2>/dev/null; rm -rf $PWD/data-e2e; }
    trap cleanup EXIT

    node_modules/.bin/wait-on -t 600000 http://127.0.0.1:5443/api/liveness

    export ENV_FILE="../.env.e2e"
    yarn workspace @hoodik/web test:e2e -- {{args}}

# Open Playwright test UI interactively (useful for debugging).
# Pass any Playwright flags as extra args, e.g. just e2e-ui --headed
e2e-ui *args:
    yarn workspace @hoodik/web test:e2e:ui -- {{args}}

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

# CI pipeline against Postgres instead of SQLite (clippy + unit + integration-pg).
# Skips the WASM/web/e2e stack — those don't care which RDBMS the server uses.
ci-test-pg: clippy test-rust-unit test-rust-integration-pg
