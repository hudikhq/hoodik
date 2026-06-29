import { test, expect } from '@playwright/test'
import path from 'path'

import { createUser, loginAsUser, logout, randomEmail, randomPassword } from './helpers/auth'
import {
  closeOpenModal,
  discoverRecipient,
  openShareDialogFor,
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

async function createSharedFolder(
  page: Parameters<typeof createUser>[0],
  recipientEmail: string
): Promise<void> {
  await page.locator('[name="create-dir"]').click()
  await page.locator('#name').fill('shared-folder')
  await page.getByRole('button', { name: 'Create', exact: true }).click()
  await expect(page.getByTestId('file-row-shared-folder')).toBeVisible({ timeout: 15_000 })
  await closeOpenModal(page)
  await openRowActions(page, 'shared-folder')
  await page.getByTestId('actions-share-account').first().click()
  await expect(page.getByTestId('share-dialog-target')).toBeVisible()
  await discoverRecipient(page, recipientEmail)
  await page.getByTestId('share-dialog-role-reader').check()
  await page.getByTestId('share-dialog-submit').click()
  await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 60_000 })
}

test.describe('Shares: Change button auto-discovers the recipient', () => {
  test('file row: Change → role picker mounts without clicking Find user', async ({ page }) => {
    const { alice, bob } = await registerTwo(page)
    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-reader').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })

    // Reopen — Bob now appears as a recipient row with [Change] [trash].
    await page.setViewportSize({ width: 1440, height: 900 })
    await openShareDialogFor(page, 'test-image.png')
    const bobRow = page.locator('[data-testid^="sharing-modal-people-row-"]').first()
    await expect(bobRow).toContainText(bob.email)
    const changeButton = bobRow.locator('[data-testid^="sharing-modal-change-role-"]')
    await expect(changeButton).toBeVisible()
    await page.screenshot({ path: '/tmp/uniform-file-modal-1440.png' })
    await changeButton.click()

    // The role picker should mount on the prefilled recipient without
    // a second Find user click — the watcher auto-fires discover().
    await expect(page.getByTestId('share-dialog-recipient-email')).toHaveText(bob.email, {
      timeout: 15_000
    })
    await expect(page.getByTestId('share-dialog-role-reader')).toBeVisible()
    await expect(page.getByTestId('share-dialog-role-editor')).toBeVisible()
    await expect(page.getByTestId('share-dialog-role-coowner')).toBeVisible()
    await page.screenshot({ path: '/tmp/uniform-change-autofind.png' })
  })

  test('folder row: Change → role picker mounts without clicking Find user', async ({ page }) => {
    const { alice, bob } = await registerTwo(page)
    await page.setViewportSize({ width: 1440, height: 900 })
    await loginAsUser(page, alice.email, alice.password)
    await createSharedFolder(page, bob.email)

    // Reopen the folder sharing modal as the owner; the FolderMembersView
    // surfaces Bob's row with the new [Change] [trash] pair.
    await closeOpenModal(page)
    await openRowActions(page, 'shared-folder')
    await page.locator('[data-testid="actions-share-account"]').first().click()
    await expect(page.getByTestId('sharing-modal-folder-members')).toBeVisible({ timeout: 15_000 })

    const change = page.locator(
      '[data-testid^="folder-members-view-row-"][data-testid$="-change"]'
    ).first()
    await expect(change).toBeVisible()
    await page.screenshot({ path: '/tmp/uniform-folder-modal-1440.png' })
    await change.click()

    await expect(page.getByTestId('share-dialog-recipient-email')).toHaveText(bob.email, {
      timeout: 15_000
    })
    await expect(page.getByTestId('share-dialog-role-reader')).toBeVisible()
  })

  test('row controls render at iPhone SE width without overflow', async ({ page }) => {
    // The row-actions dropdown trigger is hidden < sm (640px); set up the
    // share at desktop, then shrink the viewport before reopening the
    // modal. The Change/Trash pair must stay visible and fit the
    // narrowed row width.
    const { alice, bob } = await registerTwo(page)
    await loginAsUser(page, alice.email, alice.password)
    await createSharedFolder(page, bob.email)

    await closeOpenModal(page)
    await openRowActions(page, 'shared-folder')
    await page.locator('[data-testid="actions-share-account"]').first().click()
    await expect(page.getByTestId('sharing-modal-folder-members')).toBeVisible({ timeout: 15_000 })

    await page.setViewportSize({ width: 375, height: 667 })
    const change = page.locator(
      '[data-testid^="folder-members-view-row-"][data-testid$="-change"]'
    ).first()
    const revoke = page.locator(
      '[data-testid^="folder-members-view-row-"][data-testid$="-revoke"]'
    ).first()
    await expect(change).toBeVisible()
    await expect(revoke).toBeVisible()
    await page.screenshot({ path: '/tmp/uniform-folder-modal-375.png' })
  })

  test('file row controls render at iPhone SE width without overflow', async ({ page }) => {
    const { alice, bob } = await registerTwo(page)
    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-reader').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })

    await openShareDialogFor(page, 'test-image.png')
    await page.setViewportSize({ width: 375, height: 667 })
    const bobRow = page.locator('[data-testid^="sharing-modal-people-row-"]').first()
    const changeButton = bobRow.locator('[data-testid^="sharing-modal-change-role-"]')
    const revokeButton = bobRow.locator('[data-testid^="sharing-modal-revoke-"]')
    await expect(changeButton).toBeVisible()
    await expect(revokeButton).toBeVisible()
    await page.screenshot({ path: '/tmp/uniform-file-modal-375.png' })
  })
})
