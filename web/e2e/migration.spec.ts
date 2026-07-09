import { test } from '@playwright/test'

/**
 * Legacy (RSA + bcrypt) accounts auto-migrate to Curve25519 + OPAQUE on their
 * next login, re-wrapping every held file key from RSA to X25519 in the
 * process. This spec used to seed a legacy account through the register API
 * and then drive the migrating login.
 *
 * COVERAGE GAP (deliberate, not a fake). Registration is now Curve25519 +
 * OPAQUE only — the server rejects RSA signups (auth/src/data/create_user.rs) —
 * so `createUser` no longer produces a migratable account, and there is no API
 * path to create one. Seeding a legacy account from Playwright would need a
 * `users` row with a valid bcrypt password hash plus a WASM-decryptable RSA
 * `encrypted_private_key` and RSA-wrapped file/user_files rows written straight
 * into `data-e2e/sqlite.db`. bcrypt is available neither in Node here (no
 * dependency) nor in the client WASM, so that seed cannot be produced in this
 * suite without a new build dependency.
 *
 * What still covers the migration end to end:
 *   - `hoodik/tests/migration.rs` — server + full client ceremony against a
 *     `helpers::seed_legacy_user` legacy account: flips the account to
 *     curve25519/security_version=1, re-wraps keys, proves OPAQUE login and
 *     old-fingerprint signature login afterward.
 *   - `web/tests/shares/chain-resolution.test.ts` — the read-path verifiers'
 *     fallback to a since-migrated signer's pre-migration key.
 *   - `web/tests/crypto-transition.test.ts` — the client transition-certificate
 *     signatures the ceremony emits.
 *
 * To restore a browser test: add a Playwright global-setup that inserts a
 * legacy `users` row (bcrypt hash via a bundled helper) + one RSA-wrapped file
 * before the run, then drive `loginAsUser` and assert the ceremony console line
 * and that filenames still decrypt.
 */
test.describe('Legacy → Curve25519 auto-migration', () => {
  test.skip('migrates on login, files still decrypt, session survives', () => {
    // Seed path removed with RSA registration — see the block comment above.
  })
})
