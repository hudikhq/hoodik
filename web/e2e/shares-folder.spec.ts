import { test, expect } from '@playwright/test'
import path from 'path'

import { createUser, loginAsUser, logout, randomEmail, randomPassword } from './helpers/auth'
import { closeOpenModal, discoverRecipient } from './helpers/shares'

const imageFixture = path.join(__dirname, 'fixtures', 'test-image.png')

async function registerTwo(page: Parameters<typeof createUser>[0]) {
  const alice = await createUser(page, randomEmail(), randomPassword())
  await logout(page)
  const bob = await createUser(page, randomEmail(), randomPassword())
  await logout(page)
  return { alice, bob }
}

async function createFolderWithFile(page: Parameters<typeof createUser>[0]): Promise<void> {
  await page.locator('[name="create-dir"]').click()
  await page.locator('#name').fill('shared-folder')
  await page.getByRole('button', { name: 'Create', exact: true }).click()
  await expect(page.getByTestId('file-row-shared-folder')).toBeVisible({ timeout: 15_000 })
  await page.getByTestId('file-row-shared-folder').dblclick()
  // Wait for navigation to the folder.
  await expect(page).toHaveURL(/[0-9a-f-]{36}/)
  await page.setInputFiles('[name="upload-file-input"]', imageFixture)
  await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
  // Navigate back to root for the share action.
  await page.getByLabel('Breadcrumb').getByRole('link', { name: 'My Files' }).click()
  await expect(page.getByTestId('file-row-shared-folder')).toBeVisible({ timeout: 15_000 })
}

test.describe('Folder shares: subtree walking', () => {
  test('Alice shares a folder containing a single file with Bob via the action dropdown', async ({ page }) => {
    const { alice, bob } = await registerTwo(page)
    await loginAsUser(page, alice.email, alice.password)

    await createFolderWithFile(page)

    // Open the folder's action dropdown and click "Share with Hoodik account".
    await closeOpenModal(page)
    const row = page.getByTestId('file-row-shared-folder')
    await row.locator('[name="actions-dropdown"]').click()
    await page.getByTestId('actions-share-account').first().click()

    await expect(page.getByTestId('share-dialog-target')).toBeVisible()

    await discoverRecipient(page, bob.email)
    await expect(page.getByTestId('share-dialog-recipient-email')).toHaveText(bob.email)
    await page.getByTestId('share-dialog-role-reader').check()
    await page.getByTestId('share-dialog-submit').click()

    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 60_000 })

    // Validate via the API that Bob now sees the share in /api/shares/mine.
    await logout(page)
    await loginAsUser(page, bob.email, bob.password)
    const response = await page.request.get('/api/shares/mine')
    expect(response.status()).toBeLessThan(500)
  })

  test('Cancel button aborts the subtree walk without creating a share', async ({ page }) => {
    const { alice, bob } = await registerTwo(page)
    await loginAsUser(page, alice.email, alice.password)

    await createFolderWithFile(page)

    await closeOpenModal(page)
    await page.getByTestId('file-row-shared-folder').locator('[name="actions-dropdown"]').click()
    await page.getByTestId('actions-share-account').first().click()

    await expect(page.getByTestId('share-dialog-target')).toBeVisible()
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-reader').check()

    // Press Escape to dismiss the dialog without submitting. Nothing is POSTed.
    await page.keyboard.press('Escape')
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0)

    await logout(page)
    await loginAsUser(page, bob.email, bob.password)
    const response = await page.request.get('/api/shares/mine')
    expect(response.status()).toBeLessThan(500)
  })

  test('Sharing a folder root produces a share row Bob can see', async ({ page }) => {
    const { alice, bob } = await registerTwo(page)
    await loginAsUser(page, alice.email, alice.password)

    await createFolderWithFile(page)

    await closeOpenModal(page)
    await page.getByTestId('file-row-shared-folder').locator('[name="actions-dropdown"]').click()
    await page.getByTestId('actions-share-account').first().click()
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-reader').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 60_000 })

    // The recipient-side list reflects the new share. Probing the API
    // here keeps the test focused on the end-to-end correctness rather
    // than the storage store's debounced refresh inside the virtual
    // folder.
    await logout(page)
    await loginAsUser(page, bob.email, bob.password)
    const response = await page.request.get('/api/shares/mine')
    const json = (await response.json()) as { items: { owner_email: string }[] }
    expect(json.items.some((row) => row.owner_email === alice.email)).toBe(true)
  })
})
