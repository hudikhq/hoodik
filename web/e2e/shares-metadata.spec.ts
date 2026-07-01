import { test, expect } from '@playwright/test'
import path from 'path'

import { createUser, loginAsUser, logout, randomEmail, randomPassword } from './helpers/auth'
import {
  discoverRecipient,
  openShareDialogFor,
  openRowActions,
  openSharedWithMe
} from './helpers/shares'

const imageFixture = path.join(__dirname, 'fixtures', 'test-image.png')

test.describe('Recipient-side metadata propagation', () => {
  test('virtual-folder row renders real size + finished upload state', async ({ page }) => {
    const alice = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const bob = await createUser(page, randomEmail(), randomPassword())
    await logout(page)

    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-reader').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })

    await page.keyboard.press('Escape')
    await page.keyboard.press('Escape')
    await logout(page)
    await loginAsUser(page, bob.email, bob.password)
    await openSharedWithMe(page)

    const row = page.getByTestId('file-row-test-image.png')
    await expect(row).toBeVisible({ timeout: 15_000 })

    // The owner-side row stamps a size chip the moment the upload finishes;
    // the recipient row has to mirror that. "0 B" or an empty size chip is
    // the documented Fix 1 regression — the file has bytes, so the row
    // must report a non-zero size.
    const sizeText = await row.locator('div').nth(2).textContent()
    expect(sizeText?.trim()).not.toBe('-')
    expect(sizeText?.trim()).not.toBe('0 B')

    // The TableFileRow only renders the upload-progress underbar when
    // `finished_upload_at` is unset. Owner finished the upload before
    // the share, so the recipient row must not advertise progress.
    const progressBar = row.locator('+ div.border-greeny-500, + div.border-greeny-400')
    await expect(progressBar).toHaveCount(0)
  })

  test('details modal on a shared file shows real size + hashes + 100% uploaded', async ({ page }) => {
    const alice = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const bob = await createUser(page, randomEmail(), randomPassword())
    await logout(page)

    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-reader').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })

    await page.keyboard.press('Escape')
    await page.keyboard.press('Escape')
    await logout(page)
    await loginAsUser(page, bob.email, bob.password)
    await openSharedWithMe(page)

    await openRowActions(page, 'test-image.png')
    await page.locator('[name="details"]').first().click()

    // Find the Size row. The label and value share a row with two columns.
    const sizeRow = page.locator('div').filter({ hasText: /^Size$/ }).first().locator('..')
    const sizeValue = sizeRow.locator('div').nth(1)
    await expect(sizeValue).toBeVisible()
    const sizeText = await sizeValue.textContent()
    expect(sizeText?.trim()).not.toBe('-')
    expect(sizeText?.trim()).not.toBe('0 B')

    // Uploaded should render a date, not "0%". When finished_upload_at is
    // set the DetailsModal renders the formatted date; when it's unset
    // the percentage chip renders instead. The Fix 1 regression makes the
    // recipient row hit the percentage branch even though the owner
    // finished the upload — assert the date branch.
    const uploadedRow = page.locator('div').filter({ hasText: /^Uploaded$/ }).first().locator('..')
    const uploadedValue = uploadedRow.locator('div').nth(1)
    const uploadedText = await uploadedValue.textContent()
    expect(uploadedText?.trim()).not.toBe('0%')
    expect(uploadedText?.trim()).not.toMatch(/^\d+%$/)

    // The DetailsModal renders the SHA256 row once the upload-finalize
    // hash worker has caught up. The shared row's hashes come through
    // the same /api/shares/mine payload — once `sha256` is set on the
    // wire the row in the recipient surface mirrors the owner's view.
    // MD5/SHA1/BLAKE2b have no in-app computation path today so they
    // stay null on the row and the DetailsModal omits them.
    const sha256Field = page.locator('input[name="sha256"]')
    await expect(sha256Field).toBeVisible({ timeout: 15_000 })
    await expect(sha256Field).toHaveValue(/^[0-9a-f]{32,}/i)
  })

  test('incoming-share payload carries size + chunks + finished_upload_at for the recipient', async ({
    page
  }) => {
    // Round-trip the metadata through `/api/shares/mine` and prove every
    // field the recipient UI reads from an IncomingShare is on the wire.
    // Issued from inside the page context so the active session cookie
    // travels with the request — `page.request.get` shares the browser
    // jar already, but going through `fetch` from the SPA keeps the
    // test consistent with how the FE actually reads the endpoint.
    const alice = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const bob = await createUser(page, randomEmail(), randomPassword())
    await logout(page)

    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-reader').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })

    await page.keyboard.press('Escape')
    await page.keyboard.press('Escape')
    await logout(page)
    await loginAsUser(page, bob.email, bob.password)

    const payload = await page.evaluate(async () => {
      const res = await fetch('/api/shares/mine', {
        credentials: 'include',
        headers: { Accept: 'application/json' }
      })
      const ct = res.headers.get('content-type') ?? ''
      if (!res.ok || !ct.includes('application/json')) {
        throw new Error(`status=${res.status} content-type=${ct}`)
      }
      return res.json()
    })
    const items = payload.items as Array<{
      size: number | null
      chunks: number | null
      chunks_stored: number | null
      finished_upload_at: number | null
      sha256: string | null
    }>
    expect(items.length).toBeGreaterThan(0)
    const row = items[0]
    expect(row.size).not.toBeNull()
    expect(row.size).toBeGreaterThan(0)
    expect(row.chunks).not.toBeNull()
    expect(row.chunks).toBeGreaterThan(0)
    expect(row.chunks_stored).not.toBeNull()
    expect(row.finished_upload_at).not.toBeNull()
    expect(row.sha256).not.toBeNull()
  })
})
