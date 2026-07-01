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
const noteFixture = path.join(__dirname, 'fixtures', 'readonly-note.md')

async function aliceSharesWithBob(
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
  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')
}

test.describe('Virtual "Shared with me" folder under /files', () => {
  test('recipient lands on /files, opens the virtual folder, sees the shared file with a shared-by badge', async ({
    page
  }) => {
    const alice = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const bob = await createUser(page, randomEmail(), randomPassword())
    await logout(page)

    await loginAsUser(page, alice.email, alice.password)
    await aliceSharesWithBob(page, bob.email, 'reader')
    await logout(page)
    await loginAsUser(page, bob.email, bob.password)

    // The sidebar tree gets the synthetic entry too — the recipient's
    // first cue that anything has been shared with them is right there
    // in the navigation, not buried behind a click on the Files tab.
    const sidebarShared = page.getByTestId('aside-tree-shared-with-me')
    await expect(sidebarShared).toBeVisible({ timeout: 15_000 })
    await expect(sidebarShared).toContainText('Shared with me')

    await openSharedWithMe(page)

    const sharedFileRow = page.getByTestId('file-row-test-image.png')
    await expect(sharedFileRow).toBeVisible({ timeout: 15_000 })
    const badge = sharedFileRow.getByTestId('shared-by-badge')
    await expect(badge).toBeVisible()
    await expect(badge).toContainText(alice.email)
  })

  test('reader row exposes Leave + an inspectable Sharing entry but no Fork', async ({ page }) => {
    const alice = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const bob = await createUser(page, randomEmail(), randomPassword())
    await logout(page)

    await loginAsUser(page, alice.email, alice.password)
    await aliceSharesWithBob(page, bob.email, 'reader')
    await logout(page)
    await loginAsUser(page, bob.email, bob.password)
    await openSharedWithMe(page)

    await openRowActions(page, 'test-image.png')
    await expect(page.locator('[data-testid="actions-leave"]').first()).toBeVisible()
    await expect(page.locator('[data-testid="actions-fork"]')).toHaveCount(0)
    // Sharing is universal — Reader inspects the roster, mutation is
    // gated inside the modal.
    await expect(page.locator('[data-testid="actions-share-account"]').first()).toBeVisible()
  })

  test('co-owner row exposes Fork, Leave, and Sharing (reshare via SharingModal)', async ({ page }) => {
    const alice = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const bob = await createUser(page, randomEmail(), randomPassword())
    await logout(page)

    await loginAsUser(page, alice.email, alice.password)
    await aliceSharesWithBob(page, bob.email, 'co-owner')
    await logout(page)
    await loginAsUser(page, bob.email, bob.password)
    await openSharedWithMe(page)

    await openRowActions(page, 'test-image.png')
    await expect(page.locator('[data-testid="actions-fork"]').first()).toBeVisible()
    await expect(page.locator('[data-testid="actions-leave"]').first()).toBeVisible()
    await expect(page.locator('[data-testid="actions-share-account"]').first()).toBeVisible()
  })

  test('synthetic Shared with me row offers no menu and a disabled checkbox', async ({ page }) => {
    // The synthetic root id is a client-only marker — every action would
    // route a literal `__shared_with_me__` into a real storage endpoint
    // and bounce off the path-attribute validator with a 400. The
    // dropdown stays hidden; the checkbox renders for visual rhythm with
    // the rest of the listing but is disabled so it can never push the
    // synthetic id into the selection state.
    const alice = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const bob = await createUser(page, randomEmail(), randomPassword())
    await logout(page)

    await loginAsUser(page, alice.email, alice.password)
    await aliceSharesWithBob(page, bob.email, 'reader')
    await logout(page)
    await loginAsUser(page, bob.email, bob.password)

    const syntheticRow = page.getByTestId('file-row-Shared with me')
    await expect(syntheticRow).toBeVisible({ timeout: 15_000 })
    await expect(syntheticRow.locator('[name="actions-dropdown"]')).toHaveCount(0)
    const checkbox = syntheticRow.locator('input[type="checkbox"]')
    await expect(checkbox).toBeVisible()
    await expect(checkbox).toBeDisabled()
  })

  test('rename of an incoming share keeps the row in the virtual folder', async ({ page }) => {
    // Renaming a shared row used to overwrite the SPA's synthetic parent
    // pointer with the server's real `file_id` (often `null`), popping
    // the row into the recipient's own root until the next refresh. The
    // placement helper now preserves the virtual placement so the file
    // stays put. The rename route accepts non-owner writes only for
    // editable non-directories, so this test uses a markdown note shared
    // as Co-owner.
    const alice = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const bob = await createUser(page, randomEmail(), randomPassword())
    await logout(page)

    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', noteFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
    await openShareDialogFor(page, 'readonly-note.md')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-coowner').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })
    await page.keyboard.press('Escape')
    await logout(page)

    await loginAsUser(page, bob.email, bob.password)
    await openSharedWithMe(page)

    const row = page.getByTestId('file-row-readonly-note.md')
    await expect(row).toBeVisible({ timeout: 15_000 })
    // The bulk toolbar's Rename button surfaces for any single selection
    // — the dropdown's Rename entry is owner-only by convention, but the
    // bulk-action bar drops that gate so non-owners can rename what the
    // backend allows them to.
    await row.locator('input[type="checkbox"]').check()
    await page.locator('button[title="Rename file or folder"]').first().click()
    const nameInput = page.getByPlaceholder('new name')
    await nameInput.fill('renamed-note.md')
    await page.getByRole('button', { name: 'Rename', exact: true }).click()

    // The renamed row stays inside __shared_with_me__. Without the
    // placement fix the UI would put it in Bob's own root on the URL
    // `/files`.
    await expect(page.getByTestId('file-row-renamed-note.md')).toBeVisible({ timeout: 15_000 })
    await expect(page).toHaveURL(/__shared_with_me__/)
  })

  test('navigation in and out of a shared folder leaves the owned root clean', async ({
    page
  }) => {
    // Drilling into a shared folder and walking back to the recipient's
    // own root used to leak the folder into `/files` because the cached
    // entry's `file_id` got overwritten with the server's pointer (null
    // for a top-level share). Owned root must show only Bob's content +
    // the synthetic entry, never the shared folder.
    const alice = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const bob = await createUser(page, randomEmail(), randomPassword())
    await logout(page)

    await loginAsUser(page, alice.email, alice.password)
    // Create a folder + put a file in it so the share envelope has a
    // non-empty subtree (matches the working pattern in shares-folder).
    await page.locator('[name="create-dir"]').click()
    await page.locator('#name').fill('alice-docs')
    await page.getByRole('button', { name: 'Create', exact: true }).click()
    await expect(page.getByTestId('file-row-alice-docs')).toBeVisible({ timeout: 15_000 })
    await page.getByTestId('file-row-alice-docs').dblclick()
    await expect(page).toHaveURL(/[0-9a-f-]{36}/)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
    await page.getByLabel('Breadcrumb').getByRole('link', { name: 'My Files' }).click()
    await expect(page.getByTestId('file-row-alice-docs')).toBeVisible({ timeout: 15_000 })

    // Share the folder via the row dropdown (the folder share path that
    // shares-folder.spec.ts exercises end-to-end).
    await closeOpenModal(page)
    await page.getByTestId('file-row-alice-docs').locator('[name="actions-dropdown"]').click()
    await page.getByTestId('actions-share-account').first().click()
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-editor').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 60_000 })
    await logout(page)

    await loginAsUser(page, bob.email, bob.password)
    await openSharedWithMe(page)
    const sharedFolder = page.getByTestId('file-row-alice-docs')
    await expect(sharedFolder).toBeVisible({ timeout: 15_000 })

    // Double-click drills into the folder, which triggers the storage
    // store's `find(kp, folder_id)` path that previously corrupted the
    // cached row's `file_id`.
    await sharedFolder.dblclick()
    await expect(page).not.toHaveURL(/__shared_with_me__/)
    await expect(page).toHaveURL(/[0-9a-f-]{36}/)

    // Walk back to the owned root via the sidebar Files entry. After the
    // fix, the cached row keeps its `__shared_with_me__` placement so it
    // doesn't reappear under Bob's own root.
    await page.locator('aside').getByText('Files', { exact: true }).first().click()
    await expect(page).not.toHaveURL(/[0-9a-f-]{36}/)
    await expect(page).not.toHaveURL(/__shared_with_me__/)
    await expect(page.getByTestId('file-row-alice-docs')).toHaveCount(0)
  })

  test('breadcrumb root reads "Shared with me" inside the virtual folder', async ({
    page
  }) => {
    const alice = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const bob = await createUser(page, randomEmail(), randomPassword())
    await logout(page)

    await loginAsUser(page, alice.email, alice.password)
    await aliceSharesWithBob(page, bob.email, 'reader')
    await logout(page)

    await loginAsUser(page, bob.email, bob.password)
    await openSharedWithMe(page)

    // The crumb chain originates from `__shared_with_me__`, so the root
    // anchor reads "Shared with me" and routes back to the virtual folder
    // instead of dropping Bob into his (likely empty) own root.
    const root = page.getByTestId('breadcrumb-root')
    await expect(root).toBeVisible({ timeout: 15_000 })
    await expect(root).toContainText('Shared with me')
  })

  test('Reader of a shared markdown lands on the read-only preview from the virtual folder', async ({
    page
  }) => {
    const alice = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const bob = await createUser(page, randomEmail(), randomPassword())
    await logout(page)

    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', noteFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
    await openShareDialogFor(page, 'readonly-note.md')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-reader').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })
    await logout(page)

    await loginAsUser(page, bob.email, bob.password)
    await openSharedWithMe(page)

    // TableFileRow's double-click on a markdown row consults the same
    // share_role → preview-or-editor decision the search hit takes.
    // A Reader must land on `/p/<id>` so the saveable editor never
    // renders, even though the file is .md.
    await page.getByTestId('file-row-readonly-note.md').dblclick()
    await page.waitForURL(/\/p\/[0-9a-f-]{36}/, { timeout: 15_000 })
    await expect(page.locator('[name="md-save"]')).toHaveCount(0)
  })

  test('Editor of a shared markdown opens the editor from the virtual folder', async ({ page }) => {
    const alice = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const bob = await createUser(page, randomEmail(), randomPassword())
    await logout(page)

    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', noteFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
    await openShareDialogFor(page, 'readonly-note.md')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-editor').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })
    await page.keyboard.press('Escape')
    await logout(page)

    await loginAsUser(page, bob.email, bob.password)
    await openSharedWithMe(page)

    await page.getByTestId('file-row-readonly-note.md').dblclick()
    await page.waitForURL(/\/notes\/[0-9a-f-]{36}/, { timeout: 15_000 })
    await expect(page.locator('[name="md-save"]')).toBeVisible({ timeout: 15_000 })
  })
})
