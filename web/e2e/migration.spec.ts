import { test, expect, type Page } from '@playwright/test'
import { loginAsUser, logout } from './helpers/auth'

// The account, its file and its public link are inserted straight into the e2e
// database by `seed_legacy` (see the `e2e` recipe in the Justfile) because the
// register API is Curve25519 + OPAQUE only and can no longer mint a legacy
// account. These constants must match the ones the recipe passes.
const EMAIL = 'legacy-migrate@e2e.test'
const PASSWORD = 'legacy-password-1234'
const FILE_NAME = 'legacy-photo.png'

/** Collect decryption-path console/page errors for the lifetime of a page. */
function watchDecryptErrors(page: Page): string[] {
  const errors: string[] = []
  const keep = (text: string) => {
    if (/wrapping_unwrap|decrypt|undecryptable/i.test(text)) {
      errors.push(text)
    }
  }
  page.on('console', (m) => {
    if (m.type() === 'error' || m.type() === 'warning') keep(m.text())
  })
  page.on('pageerror', (e) => keep(e.message))
  return errors
}

test.describe('Legacy → Curve25519 auto-migration', () => {
  test('migrates on login; file and pre-migration link survive; account becomes OPAQUE', async ({
    page,
    browser
  }) => {
    const decryptErrors = watchDecryptErrors(page)

    // Logging in runs the client migration ceremony: re-wrap every file and
    // link key from RSA to the hybrid wrapping key, then register OPAQUE. The
    // two-key threading bug fed the identity key into a hybrid unwrap and
    // bounced the user back
    // to /auth/login — assert we land on the file browser with the file's
    // decrypted name, which only renders if the re-wrap succeeded.
    await loginAsUser(page, EMAIL, PASSWORD)
    await expect(page).not.toHaveURL(/\/auth\/login/)
    await expect(page.getByTestId(`file-row-${FILE_NAME}`)).toBeVisible()

    // A successful migration raises the one-time recovery-key notice; its overlay
    // intercepts the sidebar clicks below, so dismiss it first.
    await page.getByRole('button', { name: 'Got it' }).click()

    // The pre-migration public link still lists for its owner with its decrypted
    // name — the guard for the shipped bug where migration never re-wrapped
    // links.encrypted_link_key, stranding every pre-migration link. Navigate
    // client-side: an in-memory-only session loses its keypair on a full reload.
    await page.getByRole('listitem').filter({ hasText: 'Share' }).click()
    const table = page.getByTestId('share-hub-public-table')
    await expect(table.locator('.link-row-separator', { hasText: FILE_NAME })).toBeVisible()

    // The row's "View link" button carries the public URL with the link key in
    // its fragment — the same URL a recipient would be handed.
    const href = await table.locator('a[href^="/l/"]').first().getAttribute('href')
    expect(href).toMatch(/^\/l\/[0-9a-f-]+#[0-9a-f]+$/i)
    const publicUrl = new URL(href as string, page.url()).toString()

    // An anonymous recipient opens the same link and sees the decrypted image —
    // proving the fragment key still unwraps the metadata and file content after
    // the owner migrated.
    const anon = await browser.newContext()
    const anonPage = await anon.newPage()
    const anonErrors = watchDecryptErrors(anonPage)
    await anonPage.goto(publicUrl)
    await expect(anonPage.locator('img[name="original"]')).toBeVisible({ timeout: 30_000 })
    await expect(anonPage.locator('img[name="original"]')).toHaveAttribute('alt', FILE_NAME)
    expect(anonErrors).toEqual([])
    await anon.close()

    // The account is now migrated: a second login authenticates via OPAQUE and
    // never touches the legacy password endpoint, and the password never crosses
    // the wire. Capture only the second login's requests.
    const loginPaths: string[] = []
    const requestBodies: string[] = []
    page.on('request', (r) => {
      const { pathname } = new URL(r.url())
      if (pathname.startsWith('/api/auth/login')) loginPaths.push(pathname)
      const body = r.postData()
      if (body) requestBodies.push(body)
    })

    await logout(page)
    await loginAsUser(page, EMAIL, PASSWORD)
    await expect(page.getByTestId(`file-row-${FILE_NAME}`)).toBeVisible()

    expect(loginPaths).toContain('/api/auth/login/start')
    expect(loginPaths).toContain('/api/auth/login/finish')
    expect(loginPaths).not.toContain('/api/auth/login')
    expect(requestBodies.some((b) => b.includes(PASSWORD))).toBe(false)

    expect(decryptErrors).toEqual([])
  })
})
