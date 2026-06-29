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

test.describe('Shares: regressions', () => {
  test('B2: shared_with_count badge appears without page refresh', async ({ page }) => {
    const alice = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const bob = await createUser(page, randomEmail(), randomPassword())
    await logout(page)

    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    const row = page.getByTestId('file-row-test-image.png')
    // No badge before the share.
    await expect(row.getByTestId('shared-out-badge')).toHaveCount(0)

    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-reader').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })

    await closeOpenModal(page)
    // Badge must surface on the existing row without a reload.
    await expect(row.getByTestId('shared-out-badge')).toBeVisible({ timeout: 5_000 })
  })

  test('preview from Shared-with-me virtual root counts itself and exits back', async ({ page }) => {
    // Opening a preview from the virtual root showed 0/0 and
    // routed back to `/` on close. The preview view feeds Storage.metadata
    // into FilePreview, and the server's `file_id = null` overwrote the
    // synthetic placement the listing endpoint had computed. Fixed by
    // routing the response through `placeForRecipient` so the preview
    // sees `file_id = __shared_with_me__`.
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

    await closeOpenModal(page)
    await logout(page)

    await loginAsUser(page, bob.email, bob.password)
    await openSharedWithMe(page)
    const sharedRow = page.getByTestId('file-row-test-image.png')
    await expect(sharedRow).toBeVisible({ timeout: 15_000 })
    await sharedRow.dblclick()
    await expect(page).toHaveURL(/\/p\//, { timeout: 10_000 })

    // Sibling counter sizes the virtual folder — 1 previewable in __shared_with_me__.
    await expect(page.getByTestId('preview-counter')).toContainText('1 / 1', { timeout: 10_000 })

    // Close routes back into the virtual folder, not the recipient's own root.
    await page.locator('[name="preview-close"]').click()
    await expect(page).toHaveURL(/__shared_with_me__/, { timeout: 10_000 })
  })

  test('B3: shared image double-click routes to /p/<id> preview', async ({ page }) => {
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

    await closeOpenModal(page)
    await logout(page)

    await loginAsUser(page, bob.email, bob.password)
    await openSharedWithMe(page)
    const sharedRow = page.getByTestId('file-row-test-image.png')
    await expect(sharedRow).toBeVisible({ timeout: 15_000 })
    await sharedRow.dblclick()
    // Image preview lives under /p/<id>; details-modal fallback was the bug.
    await expect(page).toHaveURL(/\/p\//, { timeout: 10_000 })
  })

  test('public link tab is owner-only: recipients never see it', async ({ page }) => {
    // Public links are owner-side. The recipient has nothing
    // to act on (their RSA wrap of the link key doesn't exist), so the
    // Link tab is hidden on their side. The owner still sees both tabs.
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

    await closeOpenModal(page)
    await openShareDialogFor(page, 'test-image.png')
    // Owner sees both tabs.
    await expect(page.getByTestId('sharing-modal-tab-people')).toBeVisible()
    await expect(page.getByTestId('sharing-modal-tab-link')).toBeVisible()
    await page.getByTestId('sharing-modal-tab-link').click()
    const createBtn = page.getByTestId('sharing-link-create')
    if (await createBtn.isVisible().catch(() => false)) {
      await createBtn.click()
    }
    await expect(
      page.locator('[data-testid^="sharing-link-url"]').first()
    ).toBeVisible({ timeout: 10_000 })

    await closeOpenModal(page)
    await logout(page)

    await loginAsUser(page, bob.email, bob.password)
    await openSharedWithMe(page)
    await openRowActions(page, 'test-image.png')
    await page.locator('[data-testid="actions-share-account"]').first().click()
    await expect(page.getByTestId('share-dialog-target')).toBeVisible({ timeout: 10_000 })
    // Recipient sees the People surface only — no Link tab, no tab strip.
    await expect(page.getByTestId('sharing-modal-tab-link')).toHaveCount(0)
    await expect(page.getByTestId('sharing-modal-tab-people')).toHaveCount(0)
  })
})
