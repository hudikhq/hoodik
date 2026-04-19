import { test, expect } from '@playwright/test'
import { randomEmail, randomPassword, createUser } from './helpers/auth'
import {
  createNoteFromBrowser,
  createPersistedUser,
  openRawMarkdown,
  typeRawMarkdown,
  saveViaButton,
  openHistory
} from './helpers/notes'

async function setup(page: Parameters<typeof createUser>[0]) {
  await createUser(page, randomEmail(), randomPassword())
}

/**
 * A2 — versioned-chunks atomicity. Every successful save archives the
 * previous active version into `.vh-item` rows; the current content
 * stays on the file row itself (never appears in the history list).
 */

test.describe('Version history — list & restore', () => {
  test('three saves produce three historical versions shown newest-first', async ({ page }) => {
    await setup(page)
    const id = await createNoteFromBrowser(page, 'versioned.md')

    await openRawMarkdown(page)

    // Note creation lands v1 with the initial `# versioned\n` heading.
    // Save A → archives v1, active becomes v2 with "A".
    await typeRawMarkdown(page, 'A\n')
    await saveViaButton(page)

    // Save B → archives v2, active becomes v3 with "B".
    await typeRawMarkdown(page, 'B\n')
    await saveViaButton(page)

    // Save C → archives v3, active becomes v4 with "C".
    await typeRawMarkdown(page, 'C\n')
    await saveViaButton(page)

    await openHistory(page)
    await expect(page.locator('.vh-panel')).toBeVisible()

    // Three historical entries (v3, v2, v1 — newest first). The active
    // v4 with "C" lives on the file row, not in the sidebar list.
    const items = page.locator('.vh-panel .vh-item')
    await expect(items).toHaveCount(3, { timeout: 10_000 })
    await expect(items.nth(0).locator('.vh-item-version')).toHaveText('v3')
    await expect(items.nth(1).locator('.vh-item-version')).toHaveText('v2')
    await expect(items.nth(2).locator('.vh-item-version')).toHaveText('v1')

    // Active content: the raw view should show "C".
    await expect(page.locator('.md-raw-textarea')).toHaveValue(/C/)

    expect(id).toMatch(/[0-9a-f-]{36}/)
  })

  test('Restore in place swaps the active version and re-lists the rest', async ({ page }) => {
    await setup(page)
    await createNoteFromBrowser(page, 'restore-inplace.md')

    await openRawMarkdown(page)
    await typeRawMarkdown(page, 'A\n'); await saveViaButton(page)
    await typeRawMarkdown(page, 'B\n'); await saveViaButton(page)
    await typeRawMarkdown(page, 'C\n'); await saveViaButton(page)

    await openHistory(page)
    // After note creation + 3 saves: historical = [v3/B, v2/A, v1/initial].
    const items = page.locator('.vh-panel .vh-item')
    await expect(items).toHaveCount(3, { timeout: 10_000 })

    // Restore v2 — the row whose version label reads "v2". Match by
    // label rather than position so a surprise ordering change fails
    // loudly rather than silently restoring the wrong version.
    const v2Row = items.filter({ has: page.locator('.vh-item-version', { hasText: /^v2$/ }) })
    await expect(v2Row).toHaveCount(1)
    await v2Row.locator('[name="vh-restore"]').click()
    // Confirm modal
    await page.getByRole('button', { name: /Yes, restore/i }).click()

    // Active content becomes "A" (the payload of v2). Wait for the
    // editor to reload its content.
    await expect(page.locator('.md-raw-textarea')).toHaveValue(/^A\s*$/s, { timeout: 15_000 })

    // Restore archives the pre-restore active v4 ("C"), so history now
    // contains 4 historical rows: v4/C, v3/B, v2/A, v1/initial.
    await expect(items).toHaveCount(4, { timeout: 10_000 })
  })
})

test.describe('Version history — fork as new note', () => {
  test('Restore as new note creates a second file and navigates into it', async ({ page }) => {
    // Persisted session so `page.goto` back to the original file doesn't
    // bounce to the login view.
    await createPersistedUser(page)
    const originalId = await createNoteFromBrowser(page, 'fork-source.md')

    await openRawMarkdown(page)
    await typeRawMarkdown(page, 'A\n'); await saveViaButton(page)
    await typeRawMarkdown(page, 'B\n'); await saveViaButton(page)

    await openHistory(page)
    // After note creation + 2 saves: historical = [v2/A, v1/initial].
    const items = page.locator('.vh-panel .vh-item')
    await expect(items).toHaveCount(2)

    // Fork v2 ("A") — match by version label so a reordering change
    // fails loudly.
    const v2Row = items.filter({ has: page.locator('.vh-item-version', { hasText: /^v2$/ }) })
    await v2Row.locator('[name="vh-fork"]').click()

    // The page should navigate to the forked note's id. Wait on a
    // specific path change (different id) rather than a generic regex,
    // because the current URL already matches `/notes/<uuid>`.
    await page.waitForFunction(
      (orig) => {
        const m = window.location.pathname.match(/\/notes\/([0-9a-f-]{36})/)
        return !!m && m[1] !== orig
      },
      originalId,
      { timeout: 15_000 }
    )
    const newUrl = new URL(page.url())
    const forkedId = newUrl.pathname.split('/').pop() as string
    expect(forkedId).not.toBe(originalId)

    // The forked note's active content is "A".
    await openRawMarkdown(page)
    await expect(page.locator('.md-raw-textarea')).toHaveValue(/^A\s*$/s)

    // The original still has "B" — navigate back and verify.
    await page.goto(`/notes/${originalId}`)
    await openRawMarkdown(page)
    await expect(page.locator('.md-raw-textarea')).toHaveValue(/^B\s*$/s)
  })
})

test.describe('Concurrent-save conflict — 409 resolution', () => {
  // We simulate the concurrent-edit collision by intercepting the PUT
  // /content request and replaying it as 409 for non-force saves. This
  // is what "simulate via API + UI" looks like end-to-end — the conflict
  // modal, the state machine, and the force=true retry all run for
  // real, only the server response is synthesized. Runs faster and more
  // deterministically than juggling two tabs against a live pending
  // window.

  test('Overwrite path: the user picks "discard remote and overwrite" and their content persists', async ({
    page
  }) => {
    await createPersistedUser(page)
    const noteId = await createNoteFromBrowser(page, 'conflict-overwrite.md')

    await openRawMarkdown(page)
    await typeRawMarkdown(page, 'ORIGINAL\n')
    await saveViaButton(page)

    // Every non-force save → 409. Force retries pass through.
    await page.route('**/api/storage/*/content', async (route) => {
      const body = route.request().postDataJSON() as { force?: boolean } | undefined
      if (body?.force) {
        await route.continue()
      } else {
        await route.fulfill({
          status: 409,
          contentType: 'application/json',
          body: JSON.stringify({ status: 409, message: 'another_edit_is_in_progress' })
        })
      }
    })

    await typeRawMarkdown(page, 'LOCAL EDIT WINS\n')
    await page.locator('[name="md-save"]').click()

    // Conflict modal visible — choose overwrite.
    await page.getByRole('button', { name: /Discard remote and overwrite/i }).click()
    await expect(page.locator('.md-save-status')).toHaveText(/Saved/i, { timeout: 15_000 })

    await page.unroute('**/api/storage/*/content')
    await page.reload()
    await page.waitForURL(new RegExp(`/notes/${noteId}`))
    await openRawMarkdown(page)
    await expect(page.locator('.md-raw-textarea')).toHaveValue(/LOCAL EDIT WINS/)
  })

  test('Discard path: cancelling the conflict prompt drops local edits', async ({ page }) => {
    await createPersistedUser(page)
    const noteId = await createNoteFromBrowser(page, 'conflict-discard.md')

    await openRawMarkdown(page)
    await typeRawMarkdown(page, 'ORIGINAL\n')
    await saveViaButton(page)

    // Next save (non-force) will 409. Force retries pass through — not
    // that we expect one; the user picks Cancel.
    await page.route('**/api/storage/*/content', async (route) => {
      const body = route.request().postDataJSON() as { force?: boolean } | undefined
      if (body?.force) {
        await route.continue()
      } else {
        await route.fulfill({
          status: 409,
          contentType: 'application/json',
          body: JSON.stringify({ status: 409, message: 'another_edit_is_in_progress' })
        })
      }
    })

    await typeRawMarkdown(page, 'UNSAVED DRAFT\n')
    await page.locator('[name="md-save"]').click()

    // Cancel the modal — the composable drops the draft.
    await page.getByRole('button', { name: 'Cancel', exact: true }).click()

    await page.unroute('**/api/storage/*/content')
    await page.reload()
    await page.waitForURL(new RegExp(`/notes/${noteId}`))
    await openRawMarkdown(page)
    await expect(page.locator('.md-raw-textarea')).toHaveValue(/ORIGINAL/)
    await expect(page.locator('.md-raw-textarea')).not.toHaveValue(/UNSAVED DRAFT/)
  })
})
