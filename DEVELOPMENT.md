# Development

## Prerequisites

- Rust (stable, >= 1.91) via [rustup](https://rustup.rs/)
- Node.js 22 (see [.nvmrc](.nvmrc)) and [Yarn](https://yarnpkg.com/) (`npm install -g yarn`)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
- [cargo-watch](https://crates.io/crates/cargo-watch) (`cargo install cargo-watch`)
- [just](https://just.systems/) (`cargo install just`)
- Docker (for MinIO / PostgreSQL dev services)

## First-time setup

```shell
just setup
```

This installs JS dependencies, copies `.env.example` to `.env`, builds the WASM crates, and installs Playwright browsers.

## Running locally

```shell
just dev          # frontend (Vite :5173) + backend (:5443) with hot-reload
just dev-web      # Vite dev server only
just dev-api      # Rust backend only (cargo-watch)
```

`just dev` builds the WASM crate, then starts both the Vite dev server and the Rust backend with auto-reload. Files are stored on the local filesystem in `DATA_DIR` by default.

To develop with S3 storage, start MinIO separately (`just minio-up`), configure the S3 env vars in `.env`, and run `just dev` as usual.

The frontend talks to the backend at `APP_URL` (default `https://localhost:5443`). The backend serves the compiled frontend as static files in production.

## Docker services

```shell
just minio-up     # Start MinIO (S3-compatible storage, console at http://localhost:9001)
just minio-down   # Stop MinIO
just db-up        # Start PostgreSQL
just db-down      # Stop PostgreSQL
```

MinIO credentials: `minioadmin` / `minioadmin`. The `minio-init` container automatically creates a `hoodik` bucket on first start.

## Building for production

```shell
just build        # WASM -> web bundle -> Rust binary
```

## Testing

```shell
just test              # Rust unit tests + frontend unit tests
just test-rust         # All Rust tests (unit + integration)
just test-rust-unit    # Rust unit tests only
just test-web          # Frontend unit tests (Vitest)
just test-watch        # Frontend tests in watch mode

just e2e               # E2E tests (Playwright) — builds backend, starts it, runs tests, cleans up
just e2e-ui            # Interactive Playwright UI for debugging
```

## Code quality

```shell
just clippy            # Rust linting (warnings as errors)
just lint-web          # ESLint on the web frontend
just lint              # Both clippy + ESLint
just check-types       # TypeScript type-check (vue-tsc)
just check             # Clippy + TypeScript type-check
```

## Full CI pipeline

```shell
just ci-test           # Runs everything CI does:
                       # clippy -> unit tests -> integration tests ->
                       # WASM build -> frontend tests -> frontend build -> E2E
```
