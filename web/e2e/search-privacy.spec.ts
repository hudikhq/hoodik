import { test, expect } from '@playwright/test'

import { createUser, randomEmail, randomPassword } from './helpers/auth'
import { createNoteFromBrowser } from './helpers/notes'
import { closeOpenModal } from './helpers/shares'

/**
 * The E2E half of the search privacy contract: drive the real search box
 * against the real server and capture what actually crosses the wire.
 * The typed term must never appear in any `/api/storage/search` request —
 * only BERT tokens hashed with SHA-256 — and the search must still find
 * the file, proving the hashes line up with the index built at creation.
 *
 * "zanzibar" is all non-hex-safe on purpose: z, n, i and r cannot occur
 * in a hex digest, so a substring check on the raw body is conclusive.
 */

async function openSearchModal(page: Parameters<typeof createUser>[0]): Promise<void> {
  await closeOpenModal(page)
  await closeOpenModal(page)
  await page.getByRole('button', { name: /Search/ }).first().click()
  await expect(page.locator('input[placeholder="Search files..."]')).toBeVisible({
    timeout: 10_000
  })
}

test.describe('Search privacy', () => {
  test('the search request carries hashed tokens only, never the typed term', async ({
    page
  }) => {
    await createUser(page, randomEmail(), randomPassword())

    const noteName = 'zanzibar-plans.md'
    await createNoteFromBrowser(page, noteName)

    await page.locator('aside').locator(':text-is("Files")').first().click()
    await page.waitForURL(/^[^#]*\/$/, { timeout: 15_000 })
    await expect(page.getByTestId(`file-row-${noteName}`)).toBeVisible({ timeout: 15_000 })

    const searchBodies: string[] = []
    page.on('request', (request) => {
      if (request.url().includes('/api/storage/search')) {
        searchBodies.push(request.postData() || '')
      }
    })

    await openSearchModal(page)
    await page.locator('input[placeholder="Search files..."]').fill('zanzibar')

    const hit = page.locator('a[href*="/notes/"]').first()
    await expect(hit).toBeVisible({ timeout: 15_000 })
    await expect(hit).toContainText(noteName)

    expect(searchBodies.length).toBeGreaterThan(0)
    for (const raw of searchBodies) {
      expect(raw.toLowerCase()).not.toContain('zanzibar')

      const body = JSON.parse(raw)
      expect(body.search).toBeUndefined()
      expect(Array.isArray(body.search_tokens_hashed)).toBe(true)
      expect(body.search_tokens_hashed.length).toBeGreaterThan(0)
      for (const token of body.search_tokens_hashed) {
        expect(token).toMatch(/^[0-9a-f]{64}:\d+$/)
      }
    }
  })
})
