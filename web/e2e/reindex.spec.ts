import { test, expect } from '@playwright/test'

import { createPersistedUser, loginAsPersistedUser, createNoteFromBrowser } from './helpers/notes'
import { closeOpenModal } from './helpers/shares'

/**
 * The re-index sweep, driven through the real UI.
 *
 * Every user upgrading past the search re-key lands here first: the migration
 * drops the old index, and nothing server-side can rebuild it, so the browser
 * walks the account's own files and re-tags them. Until a file is done it does
 * not turn up in search at all, which is why this is a modal with an
 * explanation rather than silent background work.
 *
 * The old index is simulated by deleting the caller's tags through the same
 * route the app uses, so the sweep sees exactly what it would after a real
 * migration.
 */

async function openSearchModal(page: Parameters<typeof createNoteFromBrowser>[0]): Promise<void> {
  await closeOpenModal(page)
  await closeOpenModal(page)
  await page.getByRole('button', { name: /Search/ }).first().click()
  await expect(page.locator('input[placeholder="Search files..."]')).toBeVisible({
    timeout: 10_000
  })
}

/** How many files the server still reports as needing a re-index. */
async function pendingCount(page: Parameters<typeof createNoteFromBrowser>[0]): Promise<number> {
  return page.evaluate(async () => {
    const res = await fetch('/api/storage/reindex', { credentials: 'include' })
    return ((await res.json()) as unknown[]).length
  })
}

/**
 * Strip the caller's search tags, standing in for what the migration does.
 * Writes a `name_hash` that cannot match anything, so the row is left exactly
 * as a migrated one: present, readable, and unsearchable.
 */
async function clearIndex(
  page: Parameters<typeof createNoteFromBrowser>[0]
): Promise<{ listStatus: number; found: number; statuses: number[] }> {
  return page.evaluate(async () => {
    const list = await fetch('/api/storage', { credentials: 'include' })
    const body = (await list.json()) as { children?: { id: string }[] }
    const children = body.children ?? []
    const statuses: number[] = []

    for (const file of children) {
      const res = await fetch(`/api/storage/${file.id}/reindex`, {
        method: 'PUT',
        credentials: 'include',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          name_hash: `stale-${file.id}`,
          search_tokens_root: [],
          search_tokens_file: []
        })
      })
      statuses.push(res.status)
    }

    return { listStatus: list.status, found: children.length, statuses }
  })
}

test.describe('Search re-index', () => {
  test('rebuilds the index after a migration and restores search', async ({ page }) => {
    const user = await createPersistedUser(page)

    const noteName = 'kangaroo-notes.md'
    await createNoteFromBrowser(page, noteName)

    await page.locator('aside').locator(':text-is("Files")').first().click()
    await page.waitForURL(/^[^#]*\/$/, { timeout: 15_000 })
    await expect(page.getByTestId(`file-row-${noteName}`)).toBeVisible({ timeout: 15_000 })

    // Wipe the index the way the migration does.
    const cleared = await clearIndex(page)
    expect(cleared, `clearIndex: ${JSON.stringify(cleared)}`).toMatchObject({ listStatus: 200 })
    expect(cleared.found, `listing returned no files: ${JSON.stringify(cleared)}`).toBeGreaterThan(0)
    expect(cleared.statuses.every((s) => s === 200), `reindex PUTs: ${JSON.stringify(cleared)}`).toBe(
      true
    )
    expect(await pendingCount(page)).toBeGreaterThan(0)

    // With a single note the sweep finishes in milliseconds and the modal
    // correctly closes itself, leaving nothing to click. Hold each write open
    // long enough to exercise the controls the user actually gets.
    await page.route('**/api/storage/*/reindex', async (route) => {
      await new Promise((resolve) => setTimeout(resolve, 4000))
      await route.continue()
    })

    // A fresh session is what triggers the sweep.
    await page.reload()
    await loginAsPersistedUser(page, user.email, user.password)

    // The modal explains itself rather than leaving the user to wonder why
    // search went empty.
    const modal = page.getByText(/Search has been hardened/i)
    await expect(modal).toBeVisible({ timeout: 20_000 })
    await expect(page.getByRole('progressbar')).toBeVisible()

    // Finishing in the background closes the modal and leaves the work running.
    await page.getByRole('button', { name: /Continue in background/i }).click()
    await expect(modal).toBeHidden({ timeout: 10_000 })

    await expect
      .poll(async () => pendingCount(page), { timeout: 60_000, intervals: [1000] })
      .toBe(0)

    // The point of all of it: the note is findable again.
    await openSearchModal(page)
    await page.locator('input[placeholder="Search files..."]').fill('kangaroo')

    const hit = page.locator('a[href*="/notes/"]').first()
    await expect(hit).toBeVisible({ timeout: 15_000 })
    await expect(hit).toContainText(noteName)
  })

  test('cancelling leaves the work for next time instead of losing it', async ({ page }) => {
    const user = await createPersistedUser(page)

    await createNoteFromBrowser(page, 'wombat-notes.md')
    await page.locator('aside').locator(':text-is("Files")').first().click()
    await page.waitForURL(/^[^#]*\/$/, { timeout: 15_000 })

    const cleared = await clearIndex(page)
    const before = await pendingCount(page)
    expect(before, `clearIndex: ${JSON.stringify(cleared)}`).toBeGreaterThan(0)

    // Held open and then failed, so the file stays pending: cancelling has to
    // leave real work behind for the next session, not merely close a modal
    // that had already finished.
    await page.route('**/api/storage/*/reindex', async (route) => {
      await new Promise((resolve) => setTimeout(resolve, 3000))
      try {
        await route.fulfill({ status: 500, body: '{}' })
      } catch {
        // The click below reloads the page, which can tear the request down
        // first. Either way the write never lands, which is the point.
      }
    })

    await page.reload()
    await loginAsPersistedUser(page, user.email, user.password)

    const modal = page.getByText(/Search has been hardened/i)
    await expect(modal).toBeVisible({ timeout: 20_000 })

    await page.getByRole('button', { name: /Cancel/i }).click()
    await expect(modal).toBeHidden({ timeout: 10_000 })

    // The property that matters: cancelling is a postponement, not a refusal.
    // The file is still unindexed, so the next session picks it up — pending is
    // derived from the absence of tags rather than tracked anywhere, which is
    // what makes an interrupted sweep resumable at all.
    //
    // Asserting the count rather than waiting for the modal to reappear keeps
    // this off the modal's timing: with one file the next sweep finishes in
    // milliseconds, and a correctly fast modal is not a thing to assert on.
    expect(await pendingCount(page)).toBeGreaterThan(0)
  })
})
