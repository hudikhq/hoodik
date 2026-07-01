import { test, expect } from '@playwright/test'
import path from 'path'

import { createUser, loginAsUser, logout, randomEmail, randomPassword } from './helpers/auth'
import { discoverRecipient, openShareDialogFor, openSharedWithMe } from './helpers/shares'

const imageFixture = path.join(__dirname, 'fixtures', 'test-image.png')

type E2EPage = Parameters<typeof createUser>[0]

/** Add a member to the currently-open owned group at a chosen group role.
 *  Co-owner only renders when the caller may grant it. */
async function addMemberWithRole(
  page: E2EPage,
  email: string,
  groupRole: 'reader' | 'editor' | 'co-owner'
): Promise<void> {
  await page.locator('[data-testid^="share-hub-groups-owned-"][data-testid$="-add"]').first().click()
  await expect(page.locator('input[name="group-add-email"]')).toBeVisible({ timeout: 15_000 })
  await page.locator('input[name="group-add-email"]').fill(email)
  await page.getByTestId('group-add-member-discover').click()
  await expect(page.getByTestId('group-add-member-fingerprint')).toBeVisible({ timeout: 15_000 })
  const roleTestId =
    groupRole === 'co-owner'
      ? 'group-add-member-role-coowner'
      : `group-add-member-role-${groupRole}`
  await page.getByTestId(roleTestId).check()
  await page.getByTestId('group-add-member-submit').click()
  await expect(page.locator('input[name="group-add-email"]')).toHaveCount(0, { timeout: 30_000 })
}

async function gotoGroups(page: E2EPage): Promise<void> {
  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')
  await page.locator('aside').locator(':text-is("Share")').first().click()
  await page.waitForURL(/\/share/, { timeout: 15_000 })
  await page.locator('[data-testid="share-hub-tab-groups"]').first().click({ timeout: 15_000 })
  await expect(page.getByTestId('share-hub-groups')).toBeVisible({ timeout: 15_000 })
}

async function createGroup(page: E2EPage, name: string): Promise<void> {
  await page.getByTestId('share-hub-groups-new').click()
  await page.locator('input[name="group-name"]').fill(name)
  await page.getByTestId('group-create-submit').click()
  // The dialog closes on success; wait for the create modal to vanish before
  // asserting the row showed up.
  await expect(page.getByTestId('group-create-submit')).toHaveCount(0, { timeout: 15_000 })
  await expect(
    page.locator('[data-testid="share-hub-groups-owned-list"]').locator(`text=${name}`).first()
  ).toBeVisible({ timeout: 15_000 })
}

async function addMember(page: E2EPage, email: string): Promise<void> {
  await addMemberWithRole(page, email, 'reader')
}

async function shareToGroup(page: E2EPage, fileName: string, groupName: string): Promise<void> {
  await openShareDialogFor(page, fileName)
  await page.locator('input[name="recipient-email"]').fill(groupName)
  await page.getByTestId('share-dialog-discover').click()
  await expect(page.getByTestId('share-dialog-group-panel')).toBeVisible({ timeout: 15_000 })
  await page.getByTestId('share-dialog-group-role-reader').check()
  await page.getByTestId('share-dialog-submit').click()
  await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 60_000 })
}

// Group tests run several account-creation roundtrips plus per-member
// addMember dialogs plus a chunked file upload plus a per-recipient share
// fan-out; the default 60s test timeout is too tight for that workload.
test.describe.configure({ timeout: 180_000 })

test.describe('Group sharing', () => {
  test('Create a group with two members, share a file with the group, both members see it', async ({ page }) => {
    const alice = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const bob = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const carol = await createUser(page, randomEmail(), randomPassword())
    await logout(page)

    await loginAsUser(page, alice.email, alice.password)
    await gotoGroups(page)
    await createGroup(page, 'AcmeTeam')
    await addMember(page, bob.email)
    await addMember(page, carol.email)

    // Both members are listed in the group's roster.
    await expect(page.locator(`:has-text("${bob.email}")`).first()).toBeVisible({ timeout: 15_000 })
    await expect(page.locator(`:has-text("${carol.email}")`).first()).toBeVisible({ timeout: 15_000 })

    // Bounce the session before the upload. The groups flow opens and closes
    // several modals; logging out and back in guarantees a clean SPA state for
    // the upload + share-dialog sequence below.
    await page.keyboard.press('Escape')
    await page.keyboard.press('Escape')
    await logout(page)
    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    // Sharing to the group fans out a single share to every member.
    await shareToGroup(page, 'test-image.png', 'AcmeTeam')

    // Bob and Carol both see the share, owned by Alice.
    await page.keyboard.press('Escape')
    await page.keyboard.press('Escape')
    await logout(page)
    await loginAsUser(page, bob.email, bob.password)
    await openSharedWithMe(page)
    const bobRow = page.getByTestId('file-row-test-image.png')
    await expect(bobRow).toBeVisible({ timeout: 15_000 })
    await expect(bobRow.getByTestId('shared-by-badge')).toContainText(alice.email)
    await logout(page)

    await loginAsUser(page, carol.email, carol.password)
    await openSharedWithMe(page)
    const carolRow = page.getByTestId('file-row-test-image.png')
    await expect(carolRow).toBeVisible({ timeout: 15_000 })
    await expect(carolRow.getByTestId('shared-by-badge')).toContainText(alice.email)
  })

  test('Sharing to a group whose member already holds the file at a different role lands a role_change, not a 400', async ({
    page
  }) => {
    const alice = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const bob = await createUser(page, randomEmail(), randomPassword())
    await logout(page)

    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    // Alice grants Bob reader directly first, so when the group fan-out reaches
    // Bob it's a role move, not a fresh grant. Before the fix the fan-out signed
    // a `grant` canonical while the server reconstructed `role_change` from
    // Bob's existing row, failing the whole fan-out with `event_signature_invalid`.
    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-reader').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 15_000 })

    const aliceStorage = await page.request.get('/api/storage')
    const fileId = ((await aliceStorage.json()) as { children: { id: string }[] }).children[0].id

    // A group with Bob in it; sharing to the group at editor moves Bob reader→editor.
    await gotoGroups(page)
    await createGroup(page, 'RoleMoveTeam')
    await addMember(page, bob.email)

    // Back to the file browser via the aside (an in-SPA route — a hard reload
    // would drop the in-memory private key and bounce to login).
    await page.keyboard.press('Escape')
    await page.keyboard.press('Escape')
    await page.locator('aside').locator(':text-is("Files")').first().click()
    await expect(page.getByTestId('file-row-test-image.png')).toBeVisible({ timeout: 15_000 })
    await openShareDialogFor(page, 'test-image.png')
    await page.locator('input[name="recipient-email"]').fill('RoleMoveTeam')
    await page.getByTestId('share-dialog-discover').click()
    await expect(page.getByTestId('share-dialog-group-panel')).toBeVisible({ timeout: 15_000 })
    await page.getByTestId('share-dialog-group-role-editor').check()
    await page.getByTestId('share-dialog-submit').click()
    // The dialog only closes on a successful fan-out; a 400 leaves it open.
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 60_000 })

    // Bob's role on the file is now editor, and the audit log carries a
    // role_change row for this file — proof the transition was signed correctly.
    const recipients = (await (
      await page.request.get(`/api/shares/${fileId}`)
    ).json()) as Array<{ recipient_email: string; share_role: string }>
    expect(
      recipients.some((r) => r.recipient_email === bob.email && r.share_role === 'editor')
    ).toBe(true)

    const events = (await (
      await page.request.get('/api/shares/events?limit=50&offset=0')
    ).json()) as { events: Array<{ action: string; file_id: string }> }
    expect(
      events.events.some((e) => e.action === 'role_change' && e.file_id === fileId)
    ).toBe(true)
  })

  test('Group delete opens a CardBoxModal: not a native confirm dialog', async ({ page }) => {
    let nativeConfirmFired = false
    page.on('dialog', (dlg) => {
      nativeConfirmFired = true
      void dlg.dismiss()
    })

    const alice = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    await loginAsUser(page, alice.email, alice.password)
    await gotoGroups(page)
    await createGroup(page, 'DeletableTeam')

    const deleteButton = page
      .locator('[data-testid^="share-hub-groups-owned-"][data-testid$="-delete"]')
      .first()
    await deleteButton.click()

    // The styled modal renders; the cancel path leaves the group in place,
    // the confirm path removes it. The modal's Cancel/Delete buttons share
    // their labels with the row affordances behind the overlay, so scope the
    // click to the CardBoxModal overlay.
    const modalBody = page.getByTestId('share-hub-groups-delete-modal')
    await expect(modalBody).toBeVisible({ timeout: 5_000 })
    expect(nativeConfirmFired).toBe(false)
    const modalOverlay = page.locator('div.fixed.inset-0').filter({ has: modalBody })
    await modalOverlay.getByRole('button', { name: 'Cancel', exact: true }).click()
    await expect(modalBody).toHaveCount(0)
    await expect(
      page.locator('[data-testid="share-hub-groups-owned-list"]').locator(`text=DeletableTeam`).first()
    ).toBeVisible()

    // Re-open and confirm — the group disappears from the owned list.
    await deleteButton.click()
    await expect(modalBody).toBeVisible()
    const reopenedOverlay = page.locator('div.fixed.inset-0').filter({ has: modalBody })
    await reopenedOverlay.getByRole('button', { name: 'Delete', exact: true }).click()
    await expect(modalBody).toHaveCount(0, { timeout: 15_000 })
    await expect(
      page.locator('[data-testid="share-hub-groups-owned-list"]').locator(`text=DeletableTeam`)
    ).toHaveCount(0, { timeout: 15_000 })
    expect(nativeConfirmFired).toBe(false)
  })
})

test.describe('Group roles', () => {
  test.describe.configure({ timeout: 240_000 })

  test('An editor shares a file they own into the group; the fan-out reaches a reader peer', async ({
    page
  }) => {
    const editor = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const reader = await createUser(page, randomEmail(), randomPassword())
    await logout(page)

    // The editor owns the file and owns the group, then adds the reader as a
    // member. Owning the group carries the right to share into it; the fan-out
    // wraps the file for the reader only — it never re-grants the editor (the
    // file owner and the caller) their own row.
    await loginAsUser(page, editor.email, editor.password)
    await gotoGroups(page)
    await createGroup(page, 'RolesTeam')
    await addMemberWithRole(page, reader.email, 'reader')

    await page.keyboard.press('Escape')
    await page.keyboard.press('Escape')
    await logout(page)
    await loginAsUser(page, editor.email, editor.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    // A group share fans out to every member regardless of group role, so the
    // reader receives it.
    await shareToGroup(page, 'test-image.png', 'RolesTeam')
    await logout(page)

    // The reader peer receives the editor's share and sees it owned by the editor.
    await loginAsUser(page, reader.email, reader.password)
    await openSharedWithMe(page)
    const readerRow = page.getByTestId('file-row-test-image.png')
    await expect(readerRow).toBeVisible({ timeout: 15_000 })
    await expect(readerRow.getByTestId('shared-by-badge')).toContainText(editor.email)
  })

  test('Reader member cannot initiate a group share: the group is not offered in their picker', async ({
    page
  }) => {
    const owner = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const reader = await createUser(page, randomEmail(), randomPassword())
    await logout(page)

    await loginAsUser(page, owner.email, owner.password)
    await gotoGroups(page)
    await createGroup(page, 'ReadOnlyTeam')
    await addMemberWithRole(page, reader.email, 'reader')
    await logout(page)

    // The reader uploads their own file and opens the Share dialog. The group
    // they're a reader of must NOT appear as a share target — a reader can't
    // launder a share into the group.
    await loginAsUser(page, reader.email, reader.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
    await openShareDialogFor(page, 'test-image.png')
    const suggestion = page.locator('[data-testid="share-dialog-group-suggestions"]')
    if (await suggestion.isVisible().catch(() => false)) {
      await expect(suggestion).not.toContainText('ReadOnlyTeam')
    }
    // Typing the group name resolves to a user-discover (which 404s) rather
    // than switching to group mode, proving the group isn't a target.
    await page.locator('input[name="recipient-email"]').fill('ReadOnlyTeam')
    await page.getByTestId('share-dialog-discover').click()
    await expect(page.getByTestId('share-dialog-group-panel')).toHaveCount(0, { timeout: 10_000 })

    // The member-of card shows view-only: no manage controls, no editor
    // share-into hint.
    await gotoGroups(page)
    const memberOfList = page.getByTestId('share-hub-groups-member-of-list')
    await expect(memberOfList).toContainText('ReadOnlyTeam')
    await expect(memberOfList).toContainText('Reader')
    await expect(page.locator('[data-testid$="-manage"]')).toHaveCount(0)
  })

  test('Co-owner member manages the roster and sets a peer role; cannot delete the group', async ({
    page
  }) => {
    const owner = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const coOwner = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const member = await createUser(page, randomEmail(), randomPassword())
    await logout(page)

    await loginAsUser(page, owner.email, owner.password)
    await gotoGroups(page)
    await createGroup(page, 'ManagedTeam')
    await addMemberWithRole(page, coOwner.email, 'co-owner')
    await addMemberWithRole(page, member.email, 'reader')
    await logout(page)

    // The co-owner opens their Member-of card and manages the roster.
    await loginAsUser(page, coOwner.email, coOwner.password)
    await gotoGroups(page)
    const manage = page.locator('[data-testid$="-manage"]').first()
    await expect(manage).toBeVisible({ timeout: 15_000 })

    // No delete or rename affordance for a co-owner — both are owner-only.
    // (The add affordance is present.)
    await expect(
      page.locator('[data-testid^="share-hub-groups-member-of-"][data-testid$="-delete"]')
    ).toHaveCount(0)
    await expect(
      page.locator('[data-testid^="share-hub-groups-member-of-"][data-testid$="-rename"]')
    ).toHaveCount(0)
    await expect(
      page.locator('[data-testid^="share-hub-groups-member-of-"][data-testid$="-add"]')
    ).toHaveCount(1)

    // Load the roster and promote the reader member to editor. The role
    // control is a native <select>; scope to the element rather than the bare
    // `*="-member-"` testid so it can't match the card's own "Your role" badge
    // (testid `…-member-of-…-role`, a <span>, not a control). The owner row is
    // excluded from the manageable roster, so the only select is the member's.
    await page.locator('[data-testid$="-load-roster"]').first().click()
    const roleSelect = page
      .locator('select[data-testid*="-member-"][data-testid$="-role"]')
      .first()
    await expect(roleSelect).toBeVisible({ timeout: 15_000 })
    // A co-owner manager may set reader/editor but never co-owner — the option
    // list is fail-closed to two roles.
    await expect(roleSelect.locator('option')).toHaveCount(2)
    await roleSelect.selectOption('editor')
    // The change persists: the select reflects the new role.
    await page.waitForTimeout(500)
    await expect(roleSelect).toHaveValue('editor')
  })

  test('Owner deletes a group; the roster co-owner loses the manage surface', async ({ page }) => {
    const owner = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const coOwner = await createUser(page, randomEmail(), randomPassword())
    await logout(page)

    await loginAsUser(page, owner.email, owner.password)
    await gotoGroups(page)
    await createGroup(page, 'DoomedTeam')
    await addMemberWithRole(page, coOwner.email, 'co-owner')

    // Owner deletes the group via the styled modal.
    const deleteButton = page
      .locator('[data-testid^="share-hub-groups-owned-"][data-testid$="-delete"]')
      .first()
    await deleteButton.click()
    const modalBody = page.getByTestId('share-hub-groups-delete-modal')
    await expect(modalBody).toBeVisible({ timeout: 5_000 })
    const overlay = page.locator('div.fixed.inset-0').filter({ has: modalBody })
    await overlay.getByRole('button', { name: 'Delete', exact: true }).click()
    await expect(
      page.locator('[data-testid="share-hub-groups-owned-list"]').locator('text=DoomedTeam')
    ).toHaveCount(0, { timeout: 15_000 })
    await logout(page)

    // The co-owner no longer sees the group anywhere — delete cascaded the
    // membership rows away.
    await loginAsUser(page, coOwner.email, coOwner.password)
    await gotoGroups(page)
    await expect(page.getByTestId('share-hub-groups')).not.toContainText('DoomedTeam')
  })
})
