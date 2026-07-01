import { test, expect } from '@playwright/test'
import path from 'path'

import { createUser, loginAsUser, logout, randomEmail, randomPassword } from './helpers/auth'
import { closeOpenModal, discoverRecipient, openShareDialogFor } from './helpers/shares'

const imageFixture = path.join(__dirname, 'fixtures', 'test-image.png')

async function registerPair(page: Parameters<typeof createUser>[0]) {
  const alice = await createUser(page, randomEmail(), randomPassword())
  await logout(page)
  const bob = await createUser(page, randomEmail(), randomPassword())
  await logout(page)
  return { alice, bob }
}

async function uploadAndShareWithBob(
  page: Parameters<typeof createUser>[0],
  bobEmail: string,
  role: 'reader' | 'editor' | 'co-owner'
): Promise<void> {
  await page.setInputFiles('[name="upload-file-input"]', imageFixture)
  await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
  await openShareDialogFor(page, 'test-image.png')
  await discoverRecipient(page, bobEmail)
  if (role === 'editor') {
    await page.getByTestId('share-dialog-role-editor').check()
  } else if (role === 'co-owner') {
    await page.getByTestId('share-dialog-role-coowner').check()
  } else {
    await page.getByTestId('share-dialog-role-reader').check()
  }
  await page.getByTestId('share-dialog-submit').click()
  await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })
  await closeOpenModal(page)
  await closeOpenModal(page)
}

test.describe('Sharing modal renders existing recipients on every Share entry point', () => {
  test('row-actions Sharing entry: recipient row visible on reopen', async ({ page }) => {
    const { alice, bob } = await registerPair(page)
    await loginAsUser(page, alice.email, alice.password)
    await uploadAndShareWithBob(page, bob.email, 'reader')

    await page.getByTestId('file-row-test-image.png').locator('[name="actions-dropdown"]').click()
    await page.locator('[data-testid="actions-share-account"]').first().click()
    await expect(page.getByTestId('sharing-modal-tab-people')).toBeVisible({ timeout: 15_000 })
    await expect(page.getByTestId('sharing-modal-people-list')).toContainText(bob.email, {
      timeout: 15_000
    })
    await expect(page.locator('[data-testid^="sharing-modal-people-row-"]')).toHaveCount(1)
  })

  test('details-modal Share button: recipient row visible on reopen', async ({ page }) => {
    const { alice, bob } = await registerPair(page)
    await loginAsUser(page, alice.email, alice.password)
    await uploadAndShareWithBob(page, bob.email, 'editor')

    await openShareDialogFor(page, 'test-image.png')
    await expect(page.getByTestId('sharing-modal-people-list')).toContainText(bob.email, {
      timeout: 15_000
    })
  })

  test('single-selection bulk toolbar Sharing button: recipient row visible on reopen', async ({
    page
  }) => {
    const { alice, bob } = await registerPair(page)
    await loginAsUser(page, alice.email, alice.password)
    await uploadAndShareWithBob(page, bob.email, 'reader')

    await page.getByTestId('file-row-test-image.png').locator('input[type="checkbox"]').check()
    // Only one Share button surfaces from the toolbar now — the unified
    // entry that opens SharingModal.
    await expect(page.getByTestId('bulk-sharing-button')).toBeVisible()
    await page.getByTestId('bulk-sharing-button').click()
    await expect(page.getByTestId('sharing-modal-people-list')).toContainText(bob.email, {
      timeout: 15_000
    })
  })

  test('owned folder Sharing entry: members list visible on reopen', async ({ page }) => {
    const { alice, bob } = await registerPair(page)
    await loginAsUser(page, alice.email, alice.password)

    await page.locator('[name="create-dir"]').click()
    await page.locator('#name').fill('alice-folder')
    await page.getByRole('button', { name: 'Create', exact: true }).click()
    await expect(page.getByTestId('file-row-alice-folder')).toBeVisible({ timeout: 15_000 })
    await page.getByTestId('file-row-alice-folder').dblclick()
    await expect(page).toHaveURL(/[0-9a-f-]{36}/)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
    await page.getByLabel('Breadcrumb').getByRole('link', { name: 'My Files' }).click()
    await expect(page.getByTestId('file-row-alice-folder')).toBeVisible({ timeout: 15_000 })

    await closeOpenModal(page)
    await page.getByTestId('file-row-alice-folder').locator('[name="actions-dropdown"]').click()
    await page.locator('[data-testid="actions-share-account"]').first().click()
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-editor').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 60_000 })
    await closeOpenModal(page)
    await closeOpenModal(page)

    // Reopen the folder's Sharing modal — the signed members list must
    // render Bob's row without a second click.
    await page.getByTestId('file-row-alice-folder').locator('[name="actions-dropdown"]').click()
    await page.locator('[data-testid="actions-share-account"]').first().click()
    await expect(page.getByTestId('sharing-modal-folder-members')).toBeVisible({ timeout: 15_000 })
    await expect(page.getByTestId('folder-members-view-list')).toContainText(bob.email, {
      timeout: 15_000
    })
  })
})
