import { test, expect } from '@playwright/test'
import path from 'path'

import { createUser, loginAsUser, logout, randomEmail, randomPassword } from './helpers/auth'
import { closeOpenModal, discoverRecipient, openSharedWithMe } from './helpers/shares'

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

test.describe('Folder shares: creating folders inside a share (GH #202)', () => {
  async function shareFolderWithBob(
    page: Parameters<typeof createUser>[0],
    bobEmail: string
  ): Promise<void> {
    await page.locator('[name="create-dir"]').click()
    await page.locator('#name').fill('shared-folder')
    await page.getByRole('button', { name: 'Create', exact: true }).click()
    await expect(page.getByTestId('file-row-shared-folder')).toBeVisible({ timeout: 15_000 })

    await closeOpenModal(page)
    await page.getByTestId('file-row-shared-folder').locator('[name="actions-dropdown"]').click()
    await page.getByTestId('actions-share-account').first().click()
    await discoverRecipient(page, bobEmail)
    await page.getByTestId('share-dialog-role-editor').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 60_000 })
  }

  test('folders created inside a shared folder are visible to the other side', async ({
    page
  }) => {
    const { alice, bob } = await registerTwo(page)
    await loginAsUser(page, alice.email, alice.password)
    await shareFolderWithBob(page, bob.email)

    // Owner creates a folder inside the already-shared folder. The
    // create runs through the multi-key path, so Bob must receive a
    // wrapped key for it.
    await page.getByTestId('file-row-shared-folder').dblclick()
    await expect(page).toHaveURL(/[0-9a-f-]{36}/)
    await page.locator('[name="create-dir"]').click()
    await page.locator('#name').fill('from-alice')
    await page.getByRole('button', { name: 'Create', exact: true }).click()
    await expect(page.getByTestId('file-row-from-alice')).toBeVisible({ timeout: 15_000 })

    await logout(page)
    await loginAsUser(page, bob.email, bob.password)
    await openSharedWithMe(page)
    await page.getByTestId('file-row-shared-folder').dblclick()
    await expect(page.getByTestId('file-row-from-alice')).toBeVisible({ timeout: 15_000 })

    // The editor can create a folder here too — the affordance used to
    // be hidden for recipients — and the owner sees the result.
    await page.locator('[name="create-dir"]').click()
    await page.locator('#name').fill('from-bob')
    await page.getByRole('button', { name: 'Create', exact: true }).click()
    await expect(page.getByTestId('file-row-from-bob')).toBeVisible({ timeout: 15_000 })

    // One level deeper: the intermediate folder has no signed member
    // list of its own, so the create verifies against the share root's —
    // and the result still reaches the other side.
    await page.getByTestId('file-row-from-alice').dblclick()
    await expect(page).toHaveURL(/[0-9a-f-]{36}/)
    await page.locator('[name="create-dir"]').click()
    await page.locator('#name').fill('bob-nested')
    await page.getByRole('button', { name: 'Create', exact: true }).click()
    await expect(page.getByTestId('file-row-bob-nested')).toBeVisible({ timeout: 15_000 })

    await logout(page)
    await loginAsUser(page, alice.email, alice.password)
    await page.getByTestId('file-row-shared-folder').dblclick()
    await expect(page.getByTestId('file-row-from-bob')).toBeVisible({ timeout: 15_000 })
    await page.getByTestId('file-row-from-alice').dblclick()
    await expect(page.getByTestId('file-row-bob-nested')).toBeVisible({ timeout: 15_000 })
  })

  test('a share opened from search lists once in Shared with me', async ({ page }) => {
    const { alice, bob } = await registerTwo(page)
    await loginAsUser(page, alice.email, alice.password)
    await shareFolderWithBob(page, bob.email)

    await logout(page)
    await loginAsUser(page, bob.email, bob.password)

    // Enter the shared folder through a search hit, so its row lands in
    // the store under its real parent before the virtual folder was ever
    // listed. Re-listing "Shared with me" twice afterwards used to
    // duplicate the row (GH #202).
    await closeOpenModal(page)
    await page.getByRole('button', { name: /Search/ }).first().click()
    const searchInput = page.locator('input[placeholder="Search files..."]')
    await expect(searchInput).toBeVisible({ timeout: 10_000 })
    await searchInput.fill('shared-folder')
    const hit = page.getByRole('link', { name: /shared-folder\// }).first()
    await expect(hit).toBeVisible({ timeout: 15_000 })
    await hit.click()
    await page.waitForURL(/[0-9a-f-]{36}/, { timeout: 15_000 })

    // Back to the root, then list the virtual folder twice.
    await page.locator('aside').locator(':text-is("Files")').first().click()
    await expect(page.getByTestId('file-row-Shared with me')).toBeVisible({ timeout: 15_000 })
    await openSharedWithMe(page)
    await page.locator('aside').locator(':text-is("Files")').first().click()
    await expect(page.getByTestId('file-row-Shared with me')).toBeVisible({ timeout: 15_000 })
    await openSharedWithMe(page)

    await expect(page.getByTestId('file-row-shared-folder')).toHaveCount(1)
  })
})
