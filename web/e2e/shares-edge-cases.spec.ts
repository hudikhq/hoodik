import { test, expect } from '@playwright/test'
import path from 'path'

import { createUser, loginAsUser, logout, randomEmail, randomPassword } from './helpers/auth'
import {
  closeOpenModal,
  discoverRecipient,
  openShareDialogFor,
  openRowActions,
  openSharedWithMe
} from './helpers/shares'

const imageFixture = path.join(__dirname, 'fixtures', 'test-image.png')

test.describe('Sharing edge cases', () => {
  test('decrypt failure on one shared row leaves the virtual folder navigable', async ({ page }) => {
    const alice = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const bob = await createUser(page, randomEmail(), randomPassword())
    await logout(page)

    // Alice shares two files with Bob. The corruption of the second
    // wrap key happens on Bob's side via /api/shares/mine intercept.
    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-reader').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })
    await closeOpenModal(page)

    await page.setInputFiles(
      '[name="upload-file-input"]',
      path.join(__dirname, 'fixtures', 'test-image2.png')
    )
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
    await openShareDialogFor(page, 'test-image2.png')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-reader').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })
    await closeOpenModal(page)
    await logout(page)

    await loginAsUser(page, bob.email, bob.password)

    // Intercept the recipient list and corrupt one row's encrypted_key
    // before the SPA decrypts it. The corrupted row falls back to the
    // file id as the name; the other row still renders cleanly.
    let corruptedFileId: string | null = null
    await page.route('**/api/shares/mine**', async (route) => {
      const response = await route.fetch()
      const json = (await response.json()) as {
        items?: Array<{ file_id: string; encrypted_key: string }>
      }
      if (json.items && json.items.length >= 1) {
        corruptedFileId = json.items[0].file_id
        json.items[0].encrypted_key = 'AAAA'.repeat(32)
      }
      await route.fulfill({
        status: response.status(),
        body: JSON.stringify(json),
        contentType: 'application/json'
      })
    })

    await openSharedWithMe(page)
    // The intercept corrupts items[0] of every /api/shares/mine call.
    // The endpoint orders by `shared_at DESC, created_at DESC`, so the
    // most recent share (test-image2.png) lands at items[0]; the older
    // test-image.png decrypts cleanly and renders with its plaintext
    // name.
    const goodRow = page.getByTestId('file-row-test-image.png')
    await expect(goodRow).toBeVisible({ timeout: 15_000 })

    // The corrupted row renders the file id as a name fallback (the
    // decryption catch path on the recipient surface keeps the row
    // navigable without crashing the virtual folder).
    expect(corruptedFileId).not.toBeNull()
    await expect(page.getByTestId(`file-row-${corruptedFileId}`)).toBeVisible({ timeout: 15_000 })
  })

  test('discover self surfaces a friendly inline error, not a generic 400', async ({ page }) => {
    const alice = await createUser(page, randomEmail(), randomPassword())
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, alice.email)

    const error = page.getByTestId('share-dialog-discover-error')
    await expect(error).toBeVisible({ timeout: 10_000 })
    await expect(error).toContainText("That's your email.")
    // The fingerprint row is suppressed on the self-discover path.
    await expect(page.getByTestId('share-dialog-fingerprint')).toHaveCount(0)
  })

  test('discover an unknown email surfaces "not found"', async ({ page }) => {
    await createUser(page, randomEmail(), randomPassword())
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, 'nobody@nowhere.local')

    const error = page.getByTestId('share-dialog-discover-error')
    await expect(error).toBeVisible({ timeout: 10_000 })
    await expect(error).toContainText(/couldn't find a Hoodik account/i)
    await expect(page.getByTestId('share-dialog-fingerprint')).toHaveCount(0)
  })

  test('discover rate limit kicks in after 20 requests per minute', async ({ page }) => {
    // The per-caller bucket lives in-memory on the server; each fresh
    // account gets its own bucket, so this test is self-isolating.
    await createUser(page, randomEmail(), randomPassword())

    // Burn through 21 lookups against `nobody@nowhere.local` so the
    // 404 path doesn't count against the trip. The 21st hit returns
    // 429 (DiscoverUserError 'rate_limited' on the client).
    let status = 200
    for (let i = 0; i < 21; i++) {
      const resp = await page.request.get('/api/users/discover?email=nobody-rl@nowhere.local')
      status = resp.status()
      if (status === 429) break
    }
    expect(status).toBe(429)

    // The next UI lookup surfaces the rate-limit message via the
    // documented error pill, not a generic 400 toast. We probe with a
    // valid query so the path is "rate limit overrides the DB check".
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, 'rate-limited@nowhere.local')

    const error = page.getByTestId('share-dialog-discover-error')
    await expect(error).toBeVisible({ timeout: 10_000 })
    await expect(error).toContainText(/Slow down/i)
  })

  test('trying to share with the file owner errors at discover time, not at POST time', async ({ page }) => {
    const alice = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const bob = await createUser(page, randomEmail(), randomPassword())
    await logout(page)

    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-coowner').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })
    await closeOpenModal(page)
    await logout(page)

    await loginAsUser(page, bob.email, bob.password)
    await openSharedWithMe(page)
    await openRowActions(page, 'test-image.png')
    await page.locator('[data-testid="actions-share-account"]').first().click()
    await expect(page.getByTestId('share-dialog-target')).toBeVisible({ timeout: 15_000 })

    // Bob tries to discover Alice (the file owner). The dialog must
    // bounce the lookup at discover time so the role picker never
    // renders and the submit POST never fires — submitting would 400
    // with `cannot_share_with_owner` and surface the same gate one
    // round-trip later.
    await discoverRecipient(page, alice.email)
    const error = page.getByTestId('share-dialog-discover-error')
    await expect(error).toBeVisible({ timeout: 10_000 })
    await expect(error).toContainText(/already owns this file/i)
    await expect(page.getByTestId('share-dialog-role-reader')).toHaveCount(0)
  })
})
