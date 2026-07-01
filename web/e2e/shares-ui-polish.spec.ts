import { test, expect } from '@playwright/test'
import path from 'path'

import { createUser, loginAsUser, logout, randomEmail, randomPassword } from './helpers/auth'
import { closeOpenModal } from './helpers/shares'

const imageFixture = path.join(__dirname, 'fixtures', 'test-image.png')

async function uploadImage(page: Parameters<typeof createUser>[0]): Promise<void> {
  await page.setInputFiles('[name="upload-file-input"]', imageFixture)
  await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
}

async function openFolderSharingModal(
  page: Parameters<typeof createUser>[0],
  folderName: string
): Promise<void> {
  await closeOpenModal(page)
  await page.getByTestId(`file-row-${folderName}`).locator('[name="actions-dropdown"]').click()
  await page.locator('[data-testid="actions-share-account"]').first().click()
  await expect(page.getByTestId('share-dialog-target')).toBeVisible({ timeout: 15_000 })
}

test.describe('Sharing modal: folder tabs', () => {
  test('SharingModal on a folder collapses the tab strip', async ({ page }) => {
    // Folders never expose a Public link tab; with only one surface to
    // show, the tab strip collapses and the People body renders directly.
    await createUser(page, randomEmail(), randomPassword())
    await page.locator('[name="create-dir"]').click()
    await page.locator('#name').fill('no-link-folder')
    await page.getByRole('button', { name: 'Create', exact: true }).click()
    await expect(page.getByTestId('file-row-no-link-folder')).toBeVisible({ timeout: 15_000 })

    await openFolderSharingModal(page, 'no-link-folder')
    await expect(page.getByTestId('share-dialog-submit')).toBeVisible()
    await expect(page.getByTestId('sharing-modal-tab-link')).toHaveCount(0)
    await expect(page.getByTestId('sharing-modal-tab-people')).toHaveCount(0)
  })

  test('SharingModal on a file still shows the Public link tab', async ({ page }) => {
    await createUser(page, randomEmail(), randomPassword())
    await uploadImage(page)

    await page
      .getByTestId('file-row-test-image.png')
      .locator('[name="actions-dropdown"]')
      .click()
    await page.locator('[data-testid="actions-share-account"]').first().click()
    await expect(page.getByTestId('share-dialog-target')).toBeVisible({ timeout: 15_000 })
    await expect(page.getByTestId('sharing-modal-tab-people')).toBeVisible()
    await expect(page.getByTestId('sharing-modal-tab-link')).toBeVisible()
  })
})

test.describe('Files toolbar: share icon', () => {
  test('bulk toolbar surfaces Share when a single file is selected', async ({ page }) => {
    // The toolbar previously gated the Share entry on `chunks ===
    // chunks_stored`, which fails for any zero-chunk metadata or any
    // case where the chunk counters are unset until finalize runs. The
    // gate now matches the row dropdown: directories or finished
    // uploads, both for owned and non-owned rows.
    await createUser(page, randomEmail(), randomPassword())
    await uploadImage(page)

    const row = page.getByTestId('file-row-test-image.png')
    await row.locator('input[type="checkbox"]').check()
    await expect(page.getByTestId('bulk-sharing-button')).toBeVisible()
  })

  test('bulk toolbar surfaces Share when a single folder is selected', async ({ page }) => {
    await createUser(page, randomEmail(), randomPassword())
    await page.locator('[name="create-dir"]').click()
    await page.locator('#name').fill('toolbar-folder')
    await page.getByRole('button', { name: 'Create', exact: true }).click()
    await expect(page.getByTestId('file-row-toolbar-folder')).toBeVisible({ timeout: 15_000 })

    const row = page.getByTestId('file-row-toolbar-folder')
    await row.locator('input[type="checkbox"]').check()
    await expect(page.getByTestId('bulk-sharing-button')).toBeVisible()
  })
})

test.describe('DetailsModal: title bar', () => {
  test('DetailsModal does not render a Share affordance', async ({ page }) => {
    // Sharing is reachable from the row dropdown, the bulk-action
    // toolbar, the file preview sidebar, and the public-link surface —
    // DetailsModal's title bar carries the close X only.
    await createUser(page, randomEmail(), randomPassword())
    await uploadImage(page)

    await page
      .getByTestId('file-row-test-image.png')
      .locator('[name="actions-dropdown"]')
      .click()
    await page.locator('[name="details"]').first().click()
    // The Details modal mounts on a CardBoxModal — the close button is the
    // only top-right affordance.
    await expect(page.getByTestId('details-modal-share')).toHaveCount(0)
  })
})

test.describe('Share hub: tab strip chrome', () => {
  test('tab strip hides the scrollbar at desktop widths', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 })
    await createUser(page, randomEmail(), randomPassword())

    await page.locator('aside').locator(':text-is("Share")').first().click()
    await page.waitForURL(/\/share/, { timeout: 15_000 })

    const nav = page.getByTestId('share-hub-subtabs')
    await expect(nav).toBeVisible()
    // overflow-x stays auto so narrow viewports keep the scroll, but the
    // scrollbar chrome itself is suppressed via the `scrollbar-hide`
    // utility — Firefox uses `scrollbar-width`, WebKit hides the
    // `::-webkit-scrollbar` pseudo-element.
    const styles = await nav.evaluate((el) => {
      const computed = getComputedStyle(el)
      return {
        overflowX: computed.overflowX,
        scrollbarWidth: computed.scrollbarWidth
      }
    })
    expect(styles.overflowX).toBe('auto')
    expect(styles.scrollbarWidth).toBe('none')
  })
})

test.describe('Outgoing share badge', () => {
  test('an inline share icon appears on rows the owner has shared out', async ({ page }) => {
    const alice = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const bob = await createUser(page, randomEmail(), randomPassword())
    await logout(page)

    // remember=true persists the encrypted private key across the post-
    // share reload below; without it the SPA's refresh path can't recover
    // the in-memory keypair and bounces to /auth/login.
    await loginAsUser(page, alice.email, alice.password, { remember: true })
    await uploadImage(page)

    // No share yet — the badge stays hidden.
    const row = page.getByTestId('file-row-test-image.png')
    await expect(row).toBeVisible({ timeout: 15_000 })
    await expect(row.locator('[data-testid="shared-out-badge"]')).toHaveCount(0)

    // Share with Bob; the file row reflects the new fan-out count after
    // the next listing refresh.
    await row.locator('[name="actions-dropdown"]').click()
    await page.locator('[data-testid="actions-share-account"]').first().click()
    await expect(page.getByTestId('share-dialog-target')).toBeVisible({ timeout: 15_000 })
    await page.locator('input[name="recipient-email"]').fill(bob.email)
    await page.getByTestId('share-dialog-discover').click()
    await expect(page.getByTestId('share-dialog-fingerprint')).toBeVisible({ timeout: 10_000 })
    await page.getByTestId('share-dialog-role-reader').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })

    // The SharingPeopleAdd flow doesn't optimistically bump
    // `shared_with_count` — a `find` round-trip is needed before the
    // badge appears. Reload keeps the URL and triggers a fresh listing.
    await page.reload()
    await expect(page.getByTestId('file-row-test-image.png')).toBeVisible({
      timeout: 15_000
    })
    await expect(
      page.getByTestId('file-row-test-image.png').locator('[data-testid="shared-out-badge"]')
    ).toBeVisible()
  })
})

test.describe('Synthetic Shared-with-me row', () => {
  test('the synthetic row renders a disabled checkbox', async ({ page }) => {
    // Set up two users so Bob has the Shared-with-me virtual folder.
    const alice = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const bob = await createUser(page, randomEmail(), randomPassword())
    await logout(page)

    await loginAsUser(page, alice.email, alice.password)
    await uploadImage(page)
    await page
      .getByTestId('file-row-test-image.png')
      .locator('[name="actions-dropdown"]')
      .click()
    await page.locator('[data-testid="actions-share-account"]').first().click()
    await expect(page.getByTestId('share-dialog-target')).toBeVisible({ timeout: 15_000 })
    await page.locator('input[name="recipient-email"]').fill(bob.email)
    await page.getByTestId('share-dialog-discover').click()
    await expect(page.getByTestId('share-dialog-fingerprint')).toBeVisible({ timeout: 10_000 })
    await page.getByTestId('share-dialog-role-reader').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })
    await logout(page)

    await loginAsUser(page, bob.email, bob.password)
    const row = page.getByTestId('file-row-Shared with me')
    await expect(row).toBeVisible({ timeout: 15_000 })
    const checkbox = row.locator('input[type="checkbox"]')
    await expect(checkbox).toBeVisible()
    await expect(checkbox).toBeDisabled()
  })
})
