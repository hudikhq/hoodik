import { test, expect, type Page } from '@playwright/test'

import { loginAsUser } from './helpers/auth'
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
 * Pending state cannot be manufactured over the API any more — the write
 * routes refuse both the blank marker and the legacy digest, by design — so
 * each test logs into its own pre-migration account seeded by `seed_legacy`
 * (see the `e2e` recipe in the Justfile). The crypto migration that runs on
 * first login is what blanks the account's name hashes, exactly the state a
 * real upgrading user is in when this modal greets them.
 */

const PASSWORD = 'legacy-password-1234'
const FILE_NAME = 'legacy-photo.png'

async function openSearchModal(page: Page): Promise<void> {
  await closeOpenModal(page)
  await closeOpenModal(page)
  await page.getByRole('button', { name: /Search/ }).first().click()
  await expect(page.locator('input[placeholder="Search files..."]')).toBeVisible({
    timeout: 10_000
  })
}

/** How many files the server still reports as needing a re-index. */
async function pendingCount(page: Page): Promise<number> {
  return page.evaluate(async () => {
    const res = await fetch('/api/storage/reindex', { credentials: 'include' })
    return ((await res.json()) as unknown[]).length
  })
}

/**
 * First login of a seeded legacy account: the crypto-migration ceremony runs,
 * raises the one-time recovery-key notice, and leaves the account's files
 * waiting for the re-index sweep. Re-index writes are blocked while it runs so
 * the sweep cannot quietly finish before the test is ready to observe it.
 */
async function migrateLegacyAccount(page: Page, email: string): Promise<void> {
  await page.route('**/api/storage/*/reindex', async (route) => {
    try {
      await route.fulfill({ status: 500, body: '{}' })
    } catch {
      // A reload can tear the request down first; the write never lands
      // either way, which is the point.
    }
  })

  await loginAsUser(page, email, PASSWORD)
  await expect(page).not.toHaveURL(/\/auth\/login/)
  await page.getByRole('button', { name: 'Got it' }).click()

  expect(await pendingCount(page)).toBeGreaterThan(0)
}

test.describe('Search re-index', () => {
  test('rebuilds the index after a migration and restores search', async ({ page }) => {
    const email = 'legacy-reindex-a@e2e.test'
    await migrateLegacyAccount(page, email)

    // Let the writes through, but hold each one open: with a single file the
    // sweep finishes in milliseconds and the modal correctly closes itself,
    // leaving nothing to click.
    await page.unroute('**/api/storage/*/reindex')
    await page.route('**/api/storage/*/reindex', async (route) => {
      await new Promise((resolve) => setTimeout(resolve, 4000))
      await route.continue()
    })

    // A fresh session is what triggers the sweep.
    await page.reload()
    await loginAsUser(page, email, PASSWORD)

    // The modal explains itself rather than leaving the user to wonder why
    // search went empty.
    const modal = page.getByText(/Search index upgrade/i)
    await expect(modal).toBeVisible({ timeout: 20_000 })
    await expect(page.getByRole('progressbar')).toBeVisible()

    // Finishing in the background closes the modal and leaves the work running.
    await page.getByRole('button', { name: /Continue in background/i }).click()
    await expect(modal).toBeHidden({ timeout: 10_000 })

    await expect
      .poll(async () => pendingCount(page), { timeout: 60_000, intervals: [1000] })
      .toBe(0)

    // The point of all of it: the file is findable again.
    await openSearchModal(page)
    await page.locator('input[placeholder="Search files..."]').fill('legacy')

    await expect(page.getByText(FILE_NAME).first()).toBeVisible({ timeout: 15_000 })
  })

  test('cancelling leaves the work for next time instead of losing it', async ({ page }) => {
    const email = 'legacy-reindex-b@e2e.test'
    await migrateLegacyAccount(page, email)
    const before = await pendingCount(page)
    expect(before).toBeGreaterThan(0)

    // Writes keep failing after a hold, so the file stays pending: cancelling
    // has to leave real work behind for the next session, not merely close a
    // modal that had already finished.
    await page.unroute('**/api/storage/*/reindex')
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
    await loginAsUser(page, email, PASSWORD)

    const modal = page.getByText(/Search index upgrade/i)
    await expect(modal).toBeVisible({ timeout: 20_000 })

    await page.getByRole('button', { name: /Cancel/i }).click()
    await expect(modal).toBeHidden({ timeout: 10_000 })

    // The property that matters: cancelling is a postponement, not a refusal.
    // The file is still unindexed, so the next session picks it up — pending
    // is derived from the blank name hash rather than tracked anywhere, which
    // is what makes an interrupted sweep resumable at all.
    //
    // Asserting the count rather than waiting for the modal to reappear keeps
    // this off the modal's timing: with one file the next sweep finishes in
    // milliseconds, and a correctly fast modal is not a thing to assert on.
    expect(await pendingCount(page)).toBeGreaterThan(0)
  })
})
