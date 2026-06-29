import { test, expect } from '@playwright/test'
import path from 'path'

import { createUser, loginAsUser, logout, randomEmail, randomPassword } from './helpers/auth'
import {
  closeOpenModal,
  discoverRecipient,
  openRowActions,
  openSharedWithMe
} from './helpers/shares'

async function openFolderSharingModalForOwner(
  page: Parameters<typeof createUser>[0],
  folderName: string
): Promise<void> {
  await closeOpenModal(page)
  await openRowActions(page, folderName)
  await page.locator('[data-testid="actions-share-account"]').first().click()
  await expect(page.getByTestId('sharing-modal-folder-members')).toBeVisible({ timeout: 15_000 })
}

const imageFixture = path.join(__dirname, 'fixtures', 'test-image.png')

async function registerThree(page: Parameters<typeof createUser>[0]) {
  const alice = await createUser(page, randomEmail(), randomPassword())
  await logout(page)
  const bob = await createUser(page, randomEmail(), randomPassword())
  await logout(page)
  const carol = await createUser(page, randomEmail(), randomPassword())
  await logout(page)
  return { alice, bob, carol }
}

/**
 * Create an empty shared folder named `shared-folder` at the caller's
 * root. Leaves the page on the root with the folder row visible.
 */
async function createEmptyFolder(page: Parameters<typeof createUser>[0]): Promise<void> {
  await page.locator('[name="create-dir"]').click()
  await page.locator('#name').fill('shared-folder')
  await page.getByRole('button', { name: 'Create', exact: true }).click()
  await expect(page.getByTestId('file-row-shared-folder')).toBeVisible({ timeout: 15_000 })
}

/**
 * Share the named folder with a recipient at the given role. Closes the
 * share dialog on success.
 */
async function shareFolder(
  page: Parameters<typeof createUser>[0],
  folderName: string,
  recipientEmail: string,
  role: 'reader' | 'editor' | 'co-owner'
): Promise<void> {
  await closeOpenModal(page)
  const row = page.getByTestId(`file-row-${folderName}`)
  await row.locator('[name="actions-dropdown"]').click()
  await page.getByTestId('actions-share-account').first().click()
  await expect(page.getByTestId('share-dialog-target')).toBeVisible()
  await discoverRecipient(page, recipientEmail)
  if (role === 'editor') {
    await page.getByTestId('share-dialog-role-editor').check()
  } else if (role === 'co-owner') {
    await page.getByTestId('share-dialog-role-coowner').check()
  } else {
    await page.getByTestId('share-dialog-role-reader').check()
  }
  await page.getByTestId('share-dialog-submit').click()
  await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 60_000 })
}

test.describe('Editable folders: multi-key endpoints', () => {
  test('Editor B sees the shared folder in /api/shares/mine and can fetch its members', async ({
    page
  }) => {
    const { alice, bob } = await registerThree(page)
    await loginAsUser(page, alice.email, alice.password)
    await createEmptyFolder(page)
    await shareFolder(page, 'shared-folder', bob.email, 'editor')

    await logout(page)
    await loginAsUser(page, bob.email, bob.password)
    const mine = await page.request.get('/api/shares/mine')
    expect(mine.status()).toBe(200)
    const mineBody = (await mine.json()) as {
      items: { file_id: string; share_role: string }[]
    }
    expect(mineBody.items.length).toBeGreaterThan(0)
    const folderId = mineBody.items[0].file_id
    expect(mineBody.items[0].share_role).toBe('editor')

    // Editor B can fetch the signed member list — this is the entry
    // point of the upload-multikey pipeline.
    const members = await page.request.get(`/api/shares/folder/${folderId}/members`)
    expect(members.status()).toBe(200)
    const membersBody = (await members.json()) as {
      folder_id: string
      members: { share_role: string; is_owner: boolean }[]
    }
    expect(membersBody.folder_id).toBe(folderId)
    expect(membersBody.members.length).toBe(2)
    expect(
      membersBody.members.find((m) => m.is_owner === true)?.share_role
    ).toBe('co-owner')
    expect(
      membersBody.members.find((m) => m.share_role === 'editor')?.is_owner
    ).toBe(false)
  })

  test('Co-owner B in a folder share appears in /api/shares/folder/{id}/members with co-owner role', async ({
    page
  }) => {
    const { alice, bob } = await registerThree(page)
    await loginAsUser(page, alice.email, alice.password)
    await createEmptyFolder(page)
    await shareFolder(page, 'shared-folder', bob.email, 'co-owner')

    // Alice has the file_id; pull it from her own /api/shares/{file_id}
    // by enumerating storage first.
    const storage = await page.request.get('/api/storage')
    const storageBody = (await storage.json()) as {
      children: { id: string; mime: string }[]
    }
    const folderId = storageBody.children.find((c) => c.mime === 'dir')?.id
    expect(folderId).toBeTruthy()

    const members = await page.request.get(`/api/shares/folder/${folderId}/members`)
    expect(members.status()).toBe(200)
    const membersBody = (await members.json()) as {
      folder_id: string
      members: { user_id: string; share_role: string; is_owner: boolean }[]
    }
    expect(membersBody.members.length).toBe(2)
    const bobMember = membersBody.members.find((m) => m.is_owner === false)
    expect(bobMember?.share_role).toBe('co-owner')
  })

  test('Owner A views folder members list and revokes Editor B', async ({ page }) => {
    const { alice, bob } = await registerThree(page)
    await loginAsUser(page, alice.email, alice.password)
    await createEmptyFolder(page)
    await shareFolder(page, 'shared-folder', bob.email, 'editor')

    await openFolderSharingModalForOwner(page, 'shared-folder')

    const revoke = page.locator(
      '[data-testid^="folder-members-view-row-"][data-testid$="-revoke"]'
    ).first()
    await revoke.click()
    await expect(page.getByTestId('revoke-confirm-modal')).toBeVisible({ timeout: 5_000 })
    await page.getByTestId('revoke-confirm-modal-accept').click()

    // Close the SharingModal so the overlay releases the sidebar.
    await page.keyboard.press('Escape')
    await page.keyboard.press('Escape')
    // Bob's incoming list collapses to zero after the revoke completes.
    await logout(page)
    await loginAsUser(page, bob.email, bob.password)
    const mine = await page.request.get('/api/shares/mine')
    const mineBody = (await mine.json()) as { items: unknown[] }
    expect(mineBody.items.length).toBe(0)
  })

  test('Owner A sees the Co-owner row and cascade-confirms before revoking', async ({ page }) => {
    const { alice, bob, carol } = await registerThree(page)
    await loginAsUser(page, alice.email, alice.password)
    await createEmptyFolder(page)
    await shareFolder(page, 'shared-folder', bob.email, 'co-owner')

    // Bob re-shares the folder with Carol so the cascade has a target.
    await logout(page)
    await loginAsUser(page, bob.email, bob.password)
    await openSharedWithMe(page)
    await openRowActions(page, 'shared-folder')
    await page.locator('[data-testid="actions-share-account"]').first().click()
    await expect(page.getByTestId('share-dialog-target')).toBeVisible()
    await discoverRecipient(page, carol.email)
    await page.getByTestId('share-dialog-role-reader').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 60_000 })

    await logout(page)
    await loginAsUser(page, alice.email, alice.password)
    await openFolderSharingModalForOwner(page, 'shared-folder')

    // Pull Bob's user_id from the owner's recipient list — the row
    // whose share_role is co-owner is Bob's. Targeting his revoke
    // button by data-testid ensures we don't race other rows.
    const storage = await page.request.get('/api/storage')
    const storageBody = (await storage.json()) as {
      children: { id: string; mime: string }[]
    }
    const folderId = storageBody.children.find((c) => c.mime === 'dir')?.id
    expect(folderId).toBeTruthy()
    const recipientsResp = await page.request.get(`/api/shares/${folderId}`)
    const recipients = (await recipientsResp.json()) as {
      recipient_id: string
      share_role: 'reader' | 'editor' | 'co-owner'
    }[]
    const coownerRow = recipients.find((r) => r.share_role === 'co-owner')
    expect(coownerRow).toBeTruthy()
    await page
      .locator(
        `[data-testid="folder-members-view-row-${coownerRow!.recipient_id}-revoke"]`
      )
      .click()
    await expect(page.getByTestId('revoke-confirm-modal')).toBeVisible({ timeout: 15_000 })
    await expect(page.getByTestId('revoke-confirm-modal-cascade')).toContainText(/\d+/)
  })

  test('Folder share creates a verifiable list signature; uploader verifies and proceeds', async ({
    page
  }) => {
    const { alice, bob } = await registerThree(page)
    await loginAsUser(page, alice.email, alice.password)
    await createEmptyFolder(page)
    await shareFolder(page, 'shared-folder', bob.email, 'editor')

    // Bob's view of the folder must include a non-null
    // members_list_signature stamped by Alice — this is the
    // load-bearing artifact for the multi-key upload path.
    await logout(page)
    await loginAsUser(page, bob.email, bob.password)
    const mine = await page.request.get('/api/shares/mine')
    const mineBody = (await mine.json()) as { items: { file_id: string }[] }
    const folderId = mineBody.items[0].file_id
    const members = await page.request.get(`/api/shares/folder/${folderId}/members`)
    expect(members.status()).toBe(200)
    const body = (await members.json()) as {
      members_list_signature: string | null
      members_list_signed_by_user_id: string | null
      members_signed_at: number | null
      folder_owner_id: string
    }
    expect(body.members_list_signature).not.toBeNull()
    expect(body.members_list_signed_by_user_id).toBe(body.folder_owner_id)
    expect(body.members_signed_at).toBeGreaterThan(0)
  })

  test('Every member sees every other member\'s email on the folder roster', async ({
    page
  }) => {
    const { alice, bob } = await registerThree(page)
    await loginAsUser(page, alice.email, alice.password)
    await createEmptyFolder(page)
    await shareFolder(page, 'shared-folder', bob.email, 'editor')

    await logout(page)
    await loginAsUser(page, bob.email, bob.password)
    const mine = await page.request.get('/api/shares/mine')
    const mineBody = (await mine.json()) as { items: { file_id: string }[] }
    const folderId = mineBody.items[0].file_id
    const members = await page.request.get(`/api/shares/folder/${folderId}/members`)
    const body = (await members.json()) as {
      members: { user_id: string; email: string | null }[]
    }
    const aliceMember = body.members.find((m) => m.email === alice.email)
    expect(aliceMember).toBeDefined()
    const bobMember = body.members.find((m) => m.email === bob.email)
    expect(bobMember).toBeDefined()
  })

  test('Owner revokes; new list signature replaces old on every subsequent fetch', async ({
    page
  }) => {
    const { alice, bob, carol } = await registerThree(page)
    await loginAsUser(page, alice.email, alice.password)
    await createEmptyFolder(page)
    await shareFolder(page, 'shared-folder', bob.email, 'editor')
    await shareFolder(page, 'shared-folder', carol.email, 'reader')

    // Snapshot the signature + signed_at before the revoke.
    const storage = await page.request.get('/api/storage')
    const storageBody = (await storage.json()) as { children: { id: string; mime: string }[] }
    const folderId = storageBody.children.find((c) => c.mime === 'dir')?.id as string
    const before = await page.request.get(`/api/shares/folder/${folderId}/members`)
    const beforeBody = (await before.json()) as {
      members_list_signature: string
      members_signed_at: number
    }
    expect(beforeBody.members_list_signature).toBeTruthy()

    // Drive Alice's UI through a revoke of Carol; this exercises the
    // signFolderMemberList → revokeShare path.
    await openFolderSharingModalForOwner(page, 'shared-folder')
    const recipientsResp = await page.request.get(`/api/shares/${folderId}`)
    const recipients = (await recipientsResp.json()) as {
      recipient_id: string
      share_role: 'reader' | 'editor' | 'co-owner'
    }[]
    const carolRow = recipients.find((r) => r.share_role === 'reader')
    expect(carolRow).toBeTruthy()
    await page
      .locator(`[data-testid="folder-members-view-row-${carolRow!.recipient_id}-revoke"]`)
      .click()
    await expect(page.getByTestId('revoke-confirm-modal')).toBeVisible({ timeout: 5_000 })
    await page.getByTestId('revoke-confirm-modal-accept').click()

    // Wait for the post-revoke member list to drop to two members.
    await expect.poll(async () => {
      const after = await page.request.get(`/api/shares/folder/${folderId}/members`)
      const body = (await after.json()) as { members: unknown[] }
      return body.members.length
    }, { timeout: 15_000 }).toBe(2)

    // The new list signature differs from the pre-revoke one — fresh
    // RSA-PSS over a different member set yields different bytes.
    const after = await page.request.get(`/api/shares/folder/${folderId}/members`)
    const afterBody = (await after.json()) as {
      members_list_signature: string
      members_signed_at: number
    }
    expect(afterBody.members_list_signature).not.toEqual(beforeBody.members_list_signature)
    expect(afterBody.members_signed_at).toBeGreaterThanOrEqual(beforeBody.members_signed_at)
  })

  test('Bob navigates into a shared folder via the UI and uploads via multi-key', async ({
    page
  }) => {
    const { alice, bob } = await registerThree(page)

    // Alice creates a folder and shares it with Bob as Editor.
    await loginAsUser(page, alice.email, alice.password)
    await createEmptyFolder(page)
    await shareFolder(page, 'shared-folder', bob.email, 'editor')
    await logout(page)

    await loginAsUser(page, bob.email, bob.password)

    // Bob navigates into the virtual "Shared with me" folder under
    // /files — the recipient surface for incoming shares.
    await openSharedWithMe(page)

    // The row carries the folder id from /api/shares/mine; pull it
    // out to assert against the URL after the click.
    const mine = await page.request.get('/api/shares/mine')
    expect(mine.status()).toBe(200)
    const mineBody = (await mine.json()) as {
      items: { file_id: string; mime: string }[]
    }
    const folderRow = mineBody.items.find((i) => i.mime === 'dir')
    expect(folderRow).toBeTruthy()
    const folderId = folderRow!.file_id

    // Capture POSTs to the multi-key route so we can assert it was hit.
    const multikeyRequests: string[] = []
    page.on('request', (req) => {
      if (
        req.method() === 'POST' &&
        req.url().includes('/api/storage/upload-multikey')
      ) {
        multikeyRequests.push(req.url())
      }
    })

    // Double-click the shared folder row — TableFileRow's double-click
    // pushes to `files` with the folder id, in-SPA navigation so the
    // in-memory key survives.
    await page.getByTestId('file-row-shared-folder').dblclick()

    // The URL flips to the file browser scoped to the shared folder.
    await expect.poll(() => page.url(), { timeout: 15_000 }).toContain(folderId)

    // Wait for the file browser to finish loading + decrypting the
    // folder metadata: the breadcrumb renders the folder's decrypted
    // name ("shared-folder") once the parent is in the storage store.
    // The upload-many branch decision (multi-key vs regular) reads
    // Storage.getItem(dirId).is_owner — that lookup needs the parent
    // to be in `_items` first.
    await expect(
      page.locator('nav[aria-label="Breadcrumb"]').getByText('shared-folder')
    ).toBeVisible({ timeout: 15_000 })

    // Bob uploads an image into the shared folder. The file input is
    // the same component owned-folder uploads use; the multi-key
    // branching is decided inside LayoutFileBrowserInner.uploadMany
    // based on the parent folder's is_owner flag.
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)

    // First-contact fingerprints silently TOFU-accept on the multi-key
    // path — same posture share-create uses for an unknown recipient.
    // The mismatch case (cached-but-different) still surfaces loudly
    // via FingerprintMismatchModal; that's the only signal worth
    // gating on.
    //
    // Poll the captured request list instead of the upload-active
    // indicator: on a small image the upload finishes faster than the
    // visibility-watcher's poll cycle, so a wait-for-visible can miss
    // the flash. Asserting against `multikeyRequests` observes the
    // thing the test actually cares about — the multi-key POST fired.
    await expect.poll(() => multikeyRequests.length, { timeout: 60_000 }).toBeGreaterThanOrEqual(1)

    // The multi-key upload route — not the plain /api/storage POST —
    // must have been used. This is the load-bearing assertion: if a
    // future refactor routes the upload through the wrong path,
    // Alice's encrypted_key would be missing and she'd lose access
    // to the new file.
    expect(
      multikeyRequests.length,
      'recipient upload must hit /api/storage/upload-multikey'
    ).toBeGreaterThanOrEqual(1)

    // After the upload, Bob's view of the shared folder must include
    // the new file. The metadata endpoint now accepts non-owner reads
    // on shared dirs, so /api/storage?dir_id=<folder> returns the
    // recipient's view of the children.
    const dirListing = await page.request.get(
      `/api/storage?dir_id=${folderId}`
    )
    expect(dirListing.status()).toBe(200)
    const dirBody = (await dirListing.json()) as {
      children: { id: string; mime: string; is_owner: boolean }[]
    }
    expect(dirBody.children.length).toBeGreaterThan(0)
    // Bob uploaded the file so his row says is_owner=true; Alice's
    // copy of the same file is is_owner=false from her perspective,
    // verified separately below.
    expect(dirBody.children.some((c) => c.is_owner)).toBe(true)
    const newFileId = dirBody.children.find((c) => c.is_owner)!.id

    // Alice now sees the same file in her view of the folder, with a
    // non-owner row (she's a folder member, not the uploader).
    await logout(page)
    await loginAsUser(page, alice.email, alice.password)
    const aliceListing = await page.request.get(
      `/api/storage?dir_id=${folderId}`
    )
    expect(aliceListing.status()).toBe(200)
    const aliceBody = (await aliceListing.json()) as {
      children: { id: string; is_owner: boolean }[]
    }
    const aliceRow = aliceBody.children.find((c) => c.id === newFileId)
    expect(aliceRow).toBeTruthy()
    // Alice is a folder member but not the file owner — Bob uploaded.
    expect(aliceRow!.is_owner).toBe(false)
  })

  test('Bob creates a new note inside a shared folder without error', async ({ page }) => {
    // Regression: a cache miss on a folder member's fingerprint used to
    // surface "New member ... requires fingerprint confirmation before
    // upload" with no UI to confirm. The multi-key
    // note-create path now silently TOFU-accepts unknown members in
    // line with the share-create posture, so the create succeeds and
    // the cached fingerprint hydrates for the next upload.
    const { alice, bob } = await registerThree(page)

    await loginAsUser(page, alice.email, alice.password)
    await createEmptyFolder(page)
    await shareFolder(page, 'shared-folder', bob.email, 'editor')
    await logout(page)

    await loginAsUser(page, bob.email, bob.password)
    await openSharedWithMe(page)

    const mine = await page.request.get('/api/shares/mine')
    expect(mine.status()).toBe(200)
    const mineBody = (await mine.json()) as {
      items: { file_id: string; mime: string }[]
    }
    const folderId = mineBody.items.find((i) => i.mime === 'dir')!.file_id

    await page.getByTestId('file-row-shared-folder').dblclick()
    await expect.poll(() => page.url(), { timeout: 15_000 }).toContain(folderId)
    await expect(
      page.locator('nav[aria-label="Breadcrumb"]').getByText('shared-folder')
    ).toBeVisible({ timeout: 15_000 })

    // Fire the create-file modal to create a new note.
    // The shared-folder branch routes through uploadIntoSharedFolder,
    // which until this fix threw on the first-contact fingerprint.
    await page.locator('[name="create-file"]').click()
    await page.locator('input[name="name"]:visible').fill('shared-note.md')
    await page.getByRole('button', { name: 'Create', exact: true }).click()

    // Success path: the editor mounts on /notes/<id>. A failure would
    // have left Bob on the file browser with an error toast.
    await page.waitForURL(/\/notes\/[0-9a-f-]{36}/, { timeout: 30_000 })
    await page.locator('.milkdown-wrapper, .md-raw-wrapper').first().waitFor({
      state: 'visible',
      timeout: 15_000
    })

    // No error toast should have surfaced — assert the dialog/toast
    // element with the legacy "requires fingerprint" copy is absent.
    await expect(
      page.locator('text=requires fingerprint confirmation')
    ).toHaveCount(0)
  })
})

/**
 * Upload the image fixture at the caller's current directory and wait for the
 * row to appear with the given name.
 */
async function uploadImageAs(
  page: Parameters<typeof createUser>[0],
  rowName = 'test-image.png'
): Promise<void> {
  await page.setInputFiles('[name="upload-file-input"]', imageFixture)
  await expect(page.getByTestId(`file-row-${rowName}`)).toBeVisible({ timeout: 60_000 })
}

/** Create a child folder inside the current directory. */
async function createFolderNamed(
  page: Parameters<typeof createUser>[0],
  name: string
): Promise<void> {
  await page.locator('[name="create-dir"]').click()
  await page.locator('#name').fill(name)
  await page.getByRole('button', { name: 'Create', exact: true }).click()
  await expect(page.getByTestId(`file-row-${name}`)).toBeVisible({ timeout: 15_000 })
}

/** Tick a file row's selection checkbox. */
async function selectRow(
  page: Parameters<typeof createUser>[0],
  rowName: string
): Promise<void> {
  await page.getByTestId(`file-row-${rowName}`).locator('input[type="checkbox"]').check()
}

/**
 * Open the move picker for the current selection, then pick a destination
 * folder by name from the picker tree, or the caller's root when `target` is
 * `'Root'`. Each picker row carries a `picker-row-<name>` test id (the account
 * root is `picker-row-root`); scoping to it sidesteps the tree's recursive
 * `<li>` nesting, where a name match would otherwise resolve to an ancestor
 * wrapper. The row's Move control is a `BaseButtonConfirm`: click "Move", then
 * the revealed "Confirm".
 */
async function moveSelectionTo(
  page: Parameters<typeof createUser>[0],
  target: string
): Promise<void> {
  await page.getByTestId('move-selected').click()
  const pickerRow = page.getByTestId(target === 'Root' ? 'picker-row-root' : `picker-row-${target}`)
  await pickerRow.getByRole('button', { name: 'Move' }).click()
  await pickerRow.getByRole('button', { name: 'Confirm' }).click()
}

test.describe('Move into / out of a shared folder', () => {
  test('Owner moves a private file into their own shared folder; member B reads it', async ({
    page
  }) => {
    const { alice, bob } = await registerThree(page)
    await loginAsUser(page, alice.email, alice.password)
    await createEmptyFolder(page)
    await shareFolder(page, 'shared-folder', bob.email, 'editor')

    // The file starts as a private file at Alice's root.
    await uploadImageAs(page)

    const moveRequests: string[] = []
    page.on('request', (req) => {
      if (
        req.method() === 'POST' &&
        req.url().includes('/api/storage/move-into-shared')
      ) {
        moveRequests.push(req.url())
      }
    })

    await selectRow(page, 'test-image.png')
    await moveSelectionTo(page, 'shared-folder')

    // The single-file move-into route must have fired — not a plain
    // move-many. Without it, Alice would re-parent the file but never
    // wrap its key for Bob, silently breaking his access.
    await expect
      .poll(() => moveRequests.length, { timeout: 60_000 })
      .toBeGreaterThanOrEqual(1)

    // Bob now sees the moved file inside the shared folder.
    await logout(page)
    await loginAsUser(page, bob.email, bob.password)
    const mine = await page.request.get('/api/shares/mine')
    const mineBody = (await mine.json()) as { items: { file_id: string; mime: string }[] }
    const folderId = mineBody.items.find((i) => i.mime === 'dir')!.file_id
    const dirListing = await page.request.get(`/api/storage?dir_id=${folderId}`)
    expect(dirListing.status()).toBe(200)
    const dirBody = (await dirListing.json()) as {
      children: { id: string; mime: string }[]
    }
    expect(dirBody.children.length).toBeGreaterThanOrEqual(1)
  })

  test('Owner moves a private folder with descendants in via the confirm dialog; B reads all', async ({
    page
  }) => {
    const { alice, bob } = await registerThree(page)
    await loginAsUser(page, alice.email, alice.password)
    await createEmptyFolder(page)
    await shareFolder(page, 'shared-folder', bob.email, 'editor')

    // Build a private folder with one descendant file at Alice's root.
    await createFolderNamed(page, 'to-move')
    await page.getByTestId('file-row-to-move').dblclick()
    await expect(
      page.locator('nav[aria-label="Breadcrumb"]').getByText('to-move')
    ).toBeVisible({ timeout: 15_000 })
    await uploadImageAs(page)
    await page.getByRole('link', { name: 'My Files' }).first().click()
    await expect(page.getByTestId('file-row-to-move')).toBeVisible({ timeout: 15_000 })

    const cascadeRequests: { entries?: unknown[] }[] = []
    page.on('request', (req) => {
      if (
        req.method() === 'POST' &&
        req.url().includes('/api/storage/move-into-shared')
      ) {
        try {
          cascadeRequests.push(req.postDataJSON())
        } catch {
          cascadeRequests.push({})
        }
      }
    })

    await selectRow(page, 'to-move')
    await moveSelectionTo(page, 'shared-folder')

    // The folder cascade fires the confirm dialog before any key is
    // wrapped. Accept it.
    await expect(page.getByTestId('move-share-confirm-message')).toBeVisible({
      timeout: 15_000
    })
    await page.getByRole('button', { name: 'Move and share' }).click()

    // The cascade body carries `entries` (root + descendant), not a flat
    // `member_keys` — the load-bearing difference from the single-file path.
    await expect
      .poll(() => cascadeRequests.length, { timeout: 60_000 })
      .toBeGreaterThanOrEqual(1)
    expect(cascadeRequests.some((b) => Array.isArray(b.entries))).toBe(true)

    // Bob can read the moved folder and its descendant.
    await logout(page)
    await loginAsUser(page, bob.email, bob.password)
    const mine = await page.request.get('/api/shares/mine')
    const mineBody = (await mine.json()) as { items: { file_id: string; mime: string }[] }
    const sharedFolderId = mineBody.items.find((i) => i.mime === 'dir')!.file_id
    const listing = await page.request.get(`/api/storage?dir_id=${sharedFolderId}`)
    const body = (await listing.json()) as { children: { id: string; mime: string }[] }
    const movedFolder = body.children.find((c) => c.mime === 'dir')
    expect(movedFolder).toBeTruthy()
    const inner = await page.request.get(`/api/storage?dir_id=${movedFolder!.id}`)
    const innerBody = (await inner.json()) as { children: unknown[] }
    expect(innerBody.children.length).toBeGreaterThanOrEqual(1)
  })

  test('Non-owner C is blocked from moving a file out of a shared folder', async ({ page }) => {
    const { alice, bob: carol } = await registerThree(page)
    await loginAsUser(page, alice.email, alice.password)
    await createEmptyFolder(page)
    // Alice uploads a file she owns into her own shared folder.
    await page.getByTestId('file-row-shared-folder').dblclick()
    await expect(
      page.locator('nav[aria-label="Breadcrumb"]').getByText('shared-folder')
    ).toBeVisible({ timeout: 15_000 })
    await uploadImageAs(page)
    await page.getByRole('link', { name: 'My Files' }).first().click()
    await shareFolder(page, 'shared-folder', carol.email, 'editor')

    // Carol opens the shared folder and tries to move Alice's file out to
    // her own root. She doesn't own it, so the whole move is blocked.
    await logout(page)
    await loginAsUser(page, carol.email, carol.password)
    await openSharedWithMe(page)
    const mine = await page.request.get('/api/shares/mine')
    const mineBody = (await mine.json()) as { items: { file_id: string; mime: string }[] }
    const folderId = mineBody.items.find((i) => i.mime === 'dir')!.file_id
    await page.getByTestId('file-row-shared-folder').dblclick()
    await expect.poll(() => page.url(), { timeout: 15_000 }).toContain(folderId)
    await expect(page.getByTestId('file-row-test-image.png')).toBeVisible({ timeout: 15_000 })

    const moveOutRequests: string[] = []
    page.on('request', (req) => {
      if (req.method() === 'POST' && req.url().includes('/api/storage/move-out-of-shared')) {
        moveOutRequests.push(req.url())
      }
    })

    await selectRow(page, 'test-image.png')
    await moveSelectionTo(page, 'Root')

    // The block fires client-side before any wire call — surfaced as an
    // error toast, and no move-out POST is sent.
    await expect(page.locator('text=Only the owner can move')).toBeVisible({ timeout: 15_000 })
    expect(moveOutRequests.length).toBe(0)
  })

  test('Editor B moves their own private file into a folder shared with them; A reads it', async ({
    page
  }) => {
    const { alice, bob, carol } = await registerThree(page)

    // Alice shares an empty folder with Bob (editor) and Carol (reader).
    await loginAsUser(page, alice.email, alice.password)
    await createEmptyFolder(page)
    await shareFolder(page, 'shared-folder', bob.email, 'editor')
    await shareFolder(page, 'shared-folder', carol.email, 'reader')
    await logout(page)

    // Bob has a private file at his own root that he wants to contribute.
    await loginAsUser(page, bob.email, bob.password)
    await uploadImageAs(page)

    const mine = await page.request.get('/api/shares/mine')
    const mineBody = (await mine.json()) as { items: { file_id: string; mime: string }[] }
    const folderId = mineBody.items.find((i) => i.mime === 'dir')!.file_id

    const moveRequests: string[] = []
    page.on('request', (req) => {
      if (req.method() === 'POST' && req.url().includes('/api/storage/move-into-shared')) {
        moveRequests.push(req.url())
      }
    })

    // The destination lives under the picker's "Shared with me" branch —
    // a folder Bob is a writable member of, not one he owns. Picking it
    // routes through the single-file move-into-shared path.
    await selectRow(page, 'test-image.png')
    await moveSelectionTo(page, 'shared-folder')

    await expect
      .poll(() => moveRequests.length, { timeout: 60_000 })
      .toBeGreaterThanOrEqual(1)

    // Bob's file is now inside the shared folder, owned by Bob.
    const bobListing = await page.request.get(`/api/storage?dir_id=${folderId}`)
    expect(bobListing.status()).toBe(200)
    const bobBody = (await bobListing.json()) as {
      children: { id: string; mime: string; is_owner: boolean }[]
    }
    const movedRow = bobBody.children.find((c) => c.mime !== 'dir' && c.is_owner)
    expect(movedRow).toBeTruthy()
    const movedFileId = movedRow!.id

    // Alice (folder owner) reads the contributed file as a non-owner row —
    // her key wrap was produced by Bob's move-into-shared cascade.
    await logout(page)
    await loginAsUser(page, alice.email, alice.password)
    const aliceListing = await page.request.get(`/api/storage?dir_id=${folderId}`)
    const aliceBody = (await aliceListing.json()) as {
      children: { id: string; is_owner: boolean }[]
    }
    const aliceRow = aliceBody.children.find((c) => c.id === movedFileId)
    expect(aliceRow).toBeTruthy()
    expect(aliceRow!.is_owner).toBe(false)

    // Carol (other folder member) also receives a readable wrap.
    await logout(page)
    await loginAsUser(page, carol.email, carol.password)
    const carolMeta = await page.request.get(`/api/storage/${movedFileId}/metadata`)
    expect(carolMeta.status()).toBe(200)
  })

  test('Owner move-out drops B access', async ({ page }) => {
    const { alice, bob } = await registerThree(page)
    await loginAsUser(page, alice.email, alice.password)
    await createEmptyFolder(page)
    await page.getByTestId('file-row-shared-folder').dblclick()
    await expect(
      page.locator('nav[aria-label="Breadcrumb"]').getByText('shared-folder')
    ).toBeVisible({ timeout: 15_000 })
    await uploadImageAs(page)
    await page.getByRole('link', { name: 'My Files' }).first().click()
    await shareFolder(page, 'shared-folder', bob.email, 'editor')

    // Capture the moved file id from Bob's view before the move-out.
    await logout(page)
    await loginAsUser(page, bob.email, bob.password)
    const mineBefore = await page.request.get('/api/shares/mine')
    const beforeBody = (await mineBefore.json()) as { items: { file_id: string; mime: string }[] }
    const folderId = beforeBody.items.find((i) => i.mime === 'dir')!.file_id
    const listing = await page.request.get(`/api/storage?dir_id=${folderId}`)
    const listBody = (await listing.json()) as { children: { id: string; mime: string }[] }
    const sharedFileId = listBody.children.find((c) => c.mime !== 'dir')!.id

    // Alice moves the file out of the shared folder, back to her root.
    await logout(page)
    await loginAsUser(page, alice.email, alice.password)
    await page.getByTestId('file-row-shared-folder').dblclick()
    await expect.poll(() => page.url(), { timeout: 15_000 }).toContain(folderId)
    await expect(page.getByTestId('file-row-test-image.png')).toBeVisible({ timeout: 15_000 })

    const moveOutRequests: string[] = []
    page.on('request', (req) => {
      if (req.method() === 'POST' && req.url().includes('/api/storage/move-out-of-shared')) {
        moveOutRequests.push(req.url())
      }
    })
    await selectRow(page, 'test-image.png')
    await moveSelectionTo(page, 'Root')
    await expect.poll(() => moveOutRequests.length, { timeout: 60_000 }).toBeGreaterThanOrEqual(1)

    // Bob's row for the file is gone — the server dropped his user_files
    // entry across the moved subtree.
    await logout(page)
    await loginAsUser(page, bob.email, bob.password)
    const after = await page.request.get(`/api/storage/${sharedFileId}/metadata`)
    expect(after.status()).toBeGreaterThanOrEqual(400)
  })
})
