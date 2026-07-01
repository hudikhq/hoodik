import { test, expect } from '@playwright/test'
import path from 'path'

import { createUser, loginAsUser, logout, randomEmail, randomPassword } from './helpers/auth'
import {
  closeOpenModal,
  discoverRecipient,
  openShareDialogFor,
  openSharedWithMe,
  openRowActions
} from './helpers/shares'

const imageFixture = path.join(__dirname, 'fixtures', 'test-image.png')

async function registerTwo(page: Parameters<typeof createUser>[0]) {
  const alice = await createUser(page, randomEmail(), randomPassword())
  await logout(page)
  const bob = await createUser(page, randomEmail(), randomPassword())
  await logout(page)
  return { alice, bob }
}

async function shareImageWith(
  page: Parameters<typeof createUser>[0],
  email: string,
  role: 'reader' | 'editor' | 'coowner'
): Promise<void> {
  await openShareDialogFor(page, 'test-image.png')
  await discoverRecipient(page, email)
  await page.getByTestId(`share-dialog-role-${role}`).check()
  await page.getByTestId('share-dialog-submit').click()
  await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 60_000 })
}

async function createSharedFolder(
  page: Parameters<typeof createUser>[0],
  recipientEmail: string,
  folder = 'team-folder'
): Promise<void> {
  await page.locator('[name="create-dir"]').click()
  await page.locator('#name').fill(folder)
  await page.getByRole('button', { name: 'Create', exact: true }).click()
  await expect(page.getByTestId(`file-row-${folder}`)).toBeVisible({ timeout: 15_000 })
  await closeOpenModal(page)
  await openRowActions(page, folder)
  await page.getByTestId('actions-share-account').first().click()
  await expect(page.getByTestId('share-dialog-target')).toBeVisible()
  await discoverRecipient(page, recipientEmail)
  await page.getByTestId('share-dialog-role-editor').check()
  await page.getByTestId('share-dialog-submit').click()
  await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 60_000 })
}

test.describe('Shares: UX polish', () => {
  // Pin the viewport at desktop width for every test in this suite so the
  // file-row's actions-dropdown trigger (`hidden sm:block`) is reachable.
  // Tests that need to capture a mobile screenshot call setViewportSize at
  // the end of their own body.
  test.use({ viewport: { width: 1440, height: 900 } })

  test('submit_overlay_replaces_inline_progress_bar', async ({ page }) => {
    const { alice, bob } = await registerTwo(page)
    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-reader').check()

    // Hold the share POST so the overlay stays visible long enough for a
    // deterministic assertion + screenshot. Resolved manually after we've
    // captured the mid-submit state.
    let releasePost = () => {
      /* noop until route intercepts */
    }
    const postHeld = new Promise<void>((resolve) => {
      releasePost = resolve
    })
    await page.route('**/api/shares', async (route) => {
      await postHeld
      await route.continue()
    })

    await page.getByTestId('share-dialog-submit').click()

    // The overlay paints as soon as `submitting` flips. Inline progress
    // strip never renders while the overlay is up.
    await expect(page.getByTestId('share-dialog-submit-overlay')).toBeVisible({
      timeout: 5_000
    })
    await expect(page.getByTestId('share-dialog-progress')).toHaveCount(0)
    await page.screenshot({ path: '/tmp/uxv2-1-submit-overlay-1440.png' })

    releasePost()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 60_000 })
    await page.unroute('**/api/shares')

    // Mobile capture — the row-actions trigger is `hidden sm:block`, so
    // we have to open the modal at desktop width first and only then
    // resize. The overlay element layout doesn't depend on the page
    // viewport — it tracks the modal's body.
    await openShareDialogFor(page, 'test-image.png')
    await page.setViewportSize({ width: 375, height: 667 })
    await page.screenshot({ path: '/tmp/uxv2-1-submit-overlay-375.png' })
  })

  test('change_preselects_role_and_add_files', async ({ page }) => {
    const { alice, bob } = await registerTwo(page)
    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    await shareImageWith(page, bob.email, 'editor')

    // Reopen — Change should land with the Editor radio already selected.
    await openShareDialogFor(page, 'test-image.png')
    const bobRow = page.locator('[data-testid^="sharing-modal-people-row-"]').first()
    await expect(bobRow).toContainText(bob.email)
    await bobRow.locator('[data-testid^="sharing-modal-change-role-"]').click()
    await expect(page.getByTestId('share-dialog-recipient-email')).toHaveText(bob.email, {
      timeout: 15_000
    })
    const editorRadio = page.getByTestId('share-dialog-role-editor')
    await expect(editorRadio).toBeChecked()
    await page.screenshot({ path: '/tmp/uxv2-2-change-preselect-1440.png' })

    await page.setViewportSize({ width: 375, height: 667 })
    await page.screenshot({ path: '/tmp/uxv2-2-change-preselect-375.png' })
  })

  test('change_on_folder_preselects_editor_and_add_files', async ({ page }) => {
    const { alice, bob } = await registerTwo(page)
    await loginAsUser(page, alice.email, alice.password)
    await createSharedFolder(page, bob.email)

    await closeOpenModal(page)
    await openRowActions(page, 'team-folder')
    await page.locator('[data-testid="actions-share-account"]').first().click()
    await expect(page.getByTestId('sharing-modal-folder-members')).toBeVisible({ timeout: 15_000 })

    const change = page
      .locator('[data-testid^="folder-members-view-row-"][data-testid$="-change"]')
      .first()
    await change.click()
    await expect(page.getByTestId('share-dialog-recipient-email')).toHaveText(bob.email, {
      timeout: 15_000
    })
    await expect(page.getByTestId('share-dialog-role-editor')).toBeChecked()
    const addFiles = page.getByTestId('share-dialog-folder-editable-toggle')
    await expect(addFiles).toBeChecked()
  })

  test('reader_role_auto_unchecks_add_files', async ({ page }) => {
    const { alice, bob } = await registerTwo(page)
    await loginAsUser(page, alice.email, alice.password)
    await page.locator('[name="create-dir"]').click()
    await page.locator('#name').fill('auto-uncheck-folder')
    await page.getByRole('button', { name: 'Create', exact: true }).click()
    await expect(page.getByTestId('file-row-auto-uncheck-folder')).toBeVisible({ timeout: 15_000 })

    await closeOpenModal(page)
    await openRowActions(page, 'auto-uncheck-folder')
    await page.getByTestId('actions-share-account').first().click()
    await expect(page.getByTestId('share-dialog-target')).toBeVisible()
    await discoverRecipient(page, bob.email)

    // Default is Reader; the toggle should land unchecked. Promote to
    // Editor, the toggle stays unchecked (user must opt in); flip back
    // to Reader, it stays unchecked.
    const addFiles = page.getByTestId('share-dialog-folder-editable-toggle')
    await expect(addFiles).not.toBeChecked()
    await page.getByTestId('share-dialog-role-editor').check()
    await addFiles.check()
    await expect(addFiles).toBeChecked()
    await page.getByTestId('share-dialog-role-reader').check()
    await expect(addFiles).not.toBeChecked()
    await page.screenshot({ path: '/tmp/uxv2-3-reader-uncheck-1440.png' })

    await page.setViewportSize({ width: 375, height: 667 })
    await page.screenshot({ path: '/tmp/uxv2-3-reader-uncheck-375.png' })
  })

  test('remove_label_is_short_in_dropdown', async ({ page }) => {
    const { alice, bob } = await registerTwo(page)
    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
    await shareImageWith(page, bob.email, 'reader')

    await logout(page)
    await loginAsUser(page, bob.email, bob.password)
    await openSharedWithMe(page)

    await openRowActions(page, 'test-image.png')
    const remove = page.getByTestId('actions-leave')
    await expect(remove).toBeVisible()
    await expect(remove).toHaveText('Remove')
    await page.screenshot({ path: '/tmp/uxv2-4-remove-label-1440.png' })
  })

  test('revoked_link_renders_unavailable_page_not_unlock_form', async ({ page }) => {
    const { alice } = await registerTwo(page)
    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    await openShareDialogFor(page, 'test-image.png')
    await page.getByTestId('sharing-modal-tab-link').click()

    // Create a public link and capture the URL.
    await page.getByTestId('sharing-link-create').click()
    const linkInput = page.locator('input[name="link"]').first()
    await expect(linkInput).toBeVisible({ timeout: 30_000 })
    const linkUrl = await linkInput.inputValue()
    expect(linkUrl).toContain('#')

    // Revoke the link via the panel's delete action.
    await page.getByTestId('sharing-link-remove').click()
    await page.getByRole('button', { name: 'Confirm' }).first().click()
    // Wait for the link area to clear back to the create CTA.
    await expect(page.getByTestId('sharing-link-create')).toBeVisible({ timeout: 15_000 })

    // Visit the link URL in a fresh context (signed-out).
    const newContext = await page.context().browser()!.newContext()
    const guest = await newContext.newPage()
    await guest.goto(linkUrl)
    await expect(guest.getByTestId('link-unavailable')).toBeVisible({ timeout: 15_000 })
    await expect(guest.getByText('Link expired or removed')).toBeVisible()
    await guest.screenshot({ path: '/tmp/uxv2-5-revoked-link-1440.png' })
    await guest.setViewportSize({ width: 375, height: 667 })
    await guest.screenshot({ path: '/tmp/uxv2-5-revoked-link-375.png' })
    await newContext.close()
  })
})
