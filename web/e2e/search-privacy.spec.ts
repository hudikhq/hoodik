import { test, expect } from '@playwright/test'

import { createUser, randomEmail, randomPassword } from './helpers/auth'
import { createNoteFromBrowser } from './helpers/notes'
import { closeOpenModal } from './helpers/shares'

/**
 * The E2E half of the search privacy contract: drive the real search box
 * against the real server and capture what actually crosses the wire. The
 * typed term must never appear in any `/api/storage/search` request, only
 * keyed tags, and the search must still find the file, proving the tags line
 * up with the index built at creation.
 *
 * The absent-digest assertion is the one that matters. The index used to hold
 * an unsalted SHA-256 per token, which a table over the public BERT vocabulary
 * reverses in seconds, so it is not enough for the plaintext to be missing.
 * Its digest has to be missing too.
 *
 * "zanzibar" is all non-hex-safe on purpose: z, n, i and r cannot occur in a
 * hex digest, so a substring check on the raw body is conclusive.
 */

/** SHA-256 of "zanzibar", the exact value the old index would have stored. */
async function sha256Hex(input: string): Promise<string> {
  const bytes = new TextEncoder().encode(input)
  const digest = await crypto.subtle.digest('SHA-256', bytes)

  return Array.from(new Uint8Array(digest))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('')
}

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

      // The unsalted digest of the term must not appear either: an index of
      // bare `sha256(token)` rows is rainbow-table reversible, which is why
      // tags are keyed.
      expect(raw.toLowerCase()).not.toContain(await sha256Hex('zanzibar'))

      const body = JSON.parse(raw)
      expect(body.search).toBeUndefined()
      expect(body.search_tokens_hashed).toBeUndefined()

      expect(Array.isArray(body.root_tags)).toBe(true)
      expect(body.root_tags.length).toBeGreaterThan(0)
      for (const tag of body.root_tags) {
        expect(tag).toMatch(/^[0-9a-f]{32}$/)
      }

      // A fresh account holds no incoming shares, so there is nothing to tag
      // under a file key.
      expect(body.file_tags).toEqual([])
    }
  })
})
