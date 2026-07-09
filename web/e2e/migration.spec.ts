import { test, expect } from '@playwright/test'
import { randomEmail, randomPassword, createUser, loginAsUser, logout } from './helpers/auth'
import { createNoteFromBrowser } from './helpers/notes'

/**
 * Legacy (RSA + bcrypt) accounts auto-migrate to Curve25519 + OPAQUE on their
 * next login. The instant the re-key commits, every file key the account holds
 * has been re-wrapped from RSA to X25519, so the in-memory keypair's *wrapping*
 * key — not its Ed25519 identity key — is what decrypts file metadata from then
 * on.
 *
 * This guards the regression where a post-migration metadata decrypt reached
 * for the identity key, threw `x25519_unwrap failed`, and bounced the session
 * back to the login screen with every filename rendered as a raw UUID. Simply
 * landing on the file browser after the migrating login exercises both broken
 * paths at once: the main listing decrypt and the sidebar file-tree root fetch
 * (whose failure was the one that triggered the bounce).
 */
test.describe('Legacy → Curve25519 auto-migration', () => {
  test('migrates on login, files still decrypt, session survives', async ({ page }) => {
    const email = randomEmail()
    const password = randomPassword()

    // A legacy account with one encrypted note: gives migration an owner-wrapped
    // file key to re-wrap and the file browser something to decrypt afterward.
    await createUser(page, email, password)
    const noteName = `pre-migration-${Math.floor(Math.random() * 1e9)}.md`
    await createNoteFromBrowser(page, noteName)

    // Drop the session, then log back in — this login runs the migration
    // ceremony while the plaintext password and decrypted RSA key are both
    // still in hand.
    await logout(page)

    const consoleLines: string[] = []
    page.on('console', (m) => consoleLines.push(`${m.type()}: ${m.text()}`))

    await loginAsUser(page, email, password)

    // Not bounced back to the login screen.
    await expect(page).toHaveURL(/\/$/)

    // The note's decrypted name renders — proves the post-migration listing
    // decrypt used the X25519 wrapping key. A wrong key would show its UUID.
    await expect(page.getByText(noteName).first()).toBeVisible({ timeout: 15_000 })

    // The ceremony actually ran (guards against a silent no-op making the two
    // assertions above pass trivially), and it left no unwrap failure behind.
    expect(
      consoleLines.some((l) => l.includes('migration ceremony completed successfully')),
      consoleLines.join('\n')
    ).toBe(true)
    expect(
      consoleLines.filter((l) => l.includes('x25519_unwrap')),
      consoleLines.join('\n')
    ).toHaveLength(0)
  })
})
