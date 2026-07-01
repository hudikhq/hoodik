import { test, expect } from '@playwright/test'

import { createUser, loginAsUser, logout, randomEmail, randomPassword } from './helpers/auth'
import { createNoteFromBrowser } from './helpers/notes'
import { closeOpenModal, discoverRecipient, openShareDialogFor } from './helpers/shares'

async function shareCurrentNoteWith(
  page: Parameters<typeof createUser>[0],
  noteName: string,
  recipientEmail: string,
  role: 'reader' | 'editor'
): Promise<void> {
  // The editor view doesn't expose a Sharing button directly — go back to
  // the file browser to use the same row-actions path the rest of the
  // suite leans on.
  await page.locator('aside').locator(':text-is("Files")').first().click()
  await page.waitForURL(/^[^#]*\/$/, { timeout: 15_000 })
  await expect(page.getByTestId(`file-row-${noteName}`)).toBeVisible({ timeout: 15_000 })

  await openShareDialogFor(page, noteName)
  await discoverRecipient(page, recipientEmail)
  if (role === 'editor') {
    await page.getByTestId('share-dialog-role-editor').check()
  } else {
    await page.getByTestId('share-dialog-role-reader').check()
  }
  await page.getByTestId('share-dialog-submit').click()
  await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })
  await closeOpenModal(page)
  await closeOpenModal(page)
}

async function openSearchModal(page: Parameters<typeof createUser>[0]): Promise<void> {
  await closeOpenModal(page)
  await closeOpenModal(page)
  await page.getByRole('button', { name: /Search/ }).first().click()
  await expect(page.locator('input[placeholder="Search files..."]')).toBeVisible({
    timeout: 10_000
  })
}

test.describe('Search hits: role-aware routing', () => {
  test('Reader of a shared markdown opens the read-only preview from a search hit', async ({
    page
  }) => {
    const alice = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const bob = await createUser(page, randomEmail(), randomPassword())
    await logout(page)

    await loginAsUser(page, alice.email, alice.password)
    const noteName = 'shared-search-note.md'
    await createNoteFromBrowser(page, noteName)
    await shareCurrentNoteWith(page, noteName, bob.email, 'reader')
    await logout(page)

    await loginAsUser(page, bob.email, bob.password)
    await openSearchModal(page)
    await page.locator('input[placeholder="Search files..."]').fill('shared-search-note')

    // A Reader hit routes through the preview pipeline — not the editor.
    // `SearchModalResult.canWrite` reads `share_role` from the row and
    // sends Readers to `/p/<id>` so the saveable WYSIWYG never mounts.
    const hit = page.locator('a[href*="/p/"]').first()
    await expect(hit).toBeVisible({ timeout: 15_000 })
    await expect(hit).toContainText(noteName)
    await hit.click()
    await page.waitForURL(/\/p\/[0-9a-f-]{36}/, { timeout: 15_000 })
    await expect(page).toHaveURL(/\/p\//)
    // The Save toolbar button (editor-only) must not render on the preview.
    await expect(page.locator('[name="md-save"]')).toHaveCount(0)
  })

  test('Editor of a shared markdown opens the editor from a search hit', async ({ page }) => {
    const alice = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const bob = await createUser(page, randomEmail(), randomPassword())
    await logout(page)

    await loginAsUser(page, alice.email, alice.password)
    const noteName = 'shared-editable-note.md'
    await createNoteFromBrowser(page, noteName)
    await shareCurrentNoteWith(page, noteName, bob.email, 'editor')
    await logout(page)

    await loginAsUser(page, bob.email, bob.password)
    await openSearchModal(page)
    await page.locator('input[placeholder="Search files..."]').fill('shared-editable-note')

    const hit = page.locator('a[href*="/notes/"]').first()
    await expect(hit).toBeVisible({ timeout: 15_000 })
    await expect(hit).toContainText(noteName)
    await hit.click()
    await page.waitForURL(/\/notes\/[0-9a-f-]{36}/, { timeout: 15_000 })
    await expect(page.locator('[name="md-save"]')).toBeVisible({ timeout: 15_000 })
  })
})
