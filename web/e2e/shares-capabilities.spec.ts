import { test, expect } from '@playwright/test'
import path from 'path'

import { createUser, loginAsUser, logout, randomEmail, randomPassword } from './helpers/auth'
import { closeOpenModal, discoverRecipient, openShareDialogFor } from './helpers/shares'

const imageFixture = path.join(__dirname, 'fixtures', 'test-image.png')

test.describe('Capabilities + kill switch + unread badge', () => {
  test('/api/capabilities advertises sharing.enabled with the documented role + feature set', async ({
    page
  }) => {
    await createUser(page, randomEmail(), randomPassword())
    const resp = await page.request.get('/api/capabilities')
    expect(resp.status()).toBe(200)
    const caps = (await resp.json()) as {
      sharing: { enabled: boolean; roles: string[] }
      editable_folders: boolean
      share_groups: boolean
      audit_log: boolean
      fork: boolean
    }
    expect(caps.sharing.enabled).toBe(true)
    expect(caps.sharing.roles).toEqual(expect.arrayContaining(['reader', 'editor', 'co-owner']))
    expect(caps.editable_folders).toBe(true)
    expect(caps.share_groups).toBe(true)
    expect(caps.audit_log).toBe(true)
    expect(caps.fork).toBe(true)
  })

  test('SPA hides the Sharing entry when /api/capabilities advertises sharing.enabled=false', async ({
    page
  }) => {
    // The first user on a fresh DB would be an admin, but the e2e suite
    // shares one server across many tests so promotion isn't reliable.
    // The user-visible behaviour the admin kill switch unlocks is "the
    // SPA hides the Sharing entry on the file row dropdown" — exercise
    // that path by faking the capability response and forcing a re-fetch
    // through the same Pinia mutation the SPA goes through on cold load.
    await createUser(page, randomEmail(), randomPassword())

    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
    await expect(page.getByTestId('file-row-test-image.png')).toBeVisible()

    // Baseline — Sharing is present when the capability is on.
    await page.getByTestId('file-row-test-image.png').locator('[name="actions-dropdown"]').click()
    await expect(page.locator('[data-testid="actions-share-account"]').first()).toBeVisible()
    await closeOpenModal(page)
    await page.keyboard.press('Escape')

    // Install the intercept on the SPA's network surface (browser fetch),
    // then walk through a logout + login cycle so the capabilities store
    // re-fetches under the intercept. `page.route` does not affect
    // `page.request.*` (separate APIRequestContext), so we verify the
    // intercept landed by reading the in-app store state after the next
    // mount instead of round-tripping the assertion through fetch.
    await page.route('**/api/capabilities', async (route) => {
      const response = await route.fetch()
      const json = (await response.json()) as Record<string, unknown> & {
        sharing?: { enabled: boolean }
      }
      if (json.sharing) {
        json.sharing = { ...json.sharing, enabled: false }
      } else {
        json.sharing = { enabled: false, roles: [] }
      }
      await route.fulfill({
        status: response.status(),
        body: JSON.stringify(json),
        contentType: 'application/json'
      })
    })

    // Logout + create a fresh account so the bind + capability fetch
    // both run under the route intercept. The row dropdown loses the
    // Sharing entry because `sharingEnabled` resolves to false.
    await logout(page)
    await createUser(page, randomEmail(), randomPassword())
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
    await expect(page.getByTestId('file-row-test-image.png')).toBeVisible({ timeout: 15_000 })

    await page.getByTestId('file-row-test-image.png').locator('[name="actions-dropdown"]').click()
    await expect(page.locator('[data-testid="actions-share-account"]')).toHaveCount(0)
  })

  test('Share hub renders the unread badge until the virtual folder opens', async ({
    page
  }) => {
    const alice = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const bob = await createUser(page, randomEmail(), randomPassword())
    await logout(page)

    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-reader').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })
    await logout(page)

    await loginAsUser(page, bob.email, bob.password)
    // The badge lives on the Share hub header — opening it mounts the
    // counter from the same `unreadCount` getter the sidebar removed in
    // ebbd646. The count is non-zero until the virtual folder marks
    // shares as seen.
    await page.locator('aside').locator(':text-is("Share")').first().click()
    await page.waitForURL(/\/share/, { timeout: 15_000 })
    const badge = page.getByTestId('share-hub-unread-badge')
    await expect(badge).toBeVisible({ timeout: 15_000 })
    await expect(badge).toContainText(/\d+/)

    // Opening the virtual folder is the canonical "I saw what's new"
    // action — `markSeenNow` runs in the file browser's load step and
    // the next visit to the hub renders no badge.
    await page.locator('aside').locator(':text-is("Files")').first().click()
    await page.waitForURL(/^[^#]*\/$/, { timeout: 15_000 })
    const sharedRow = page.getByTestId('file-row-Shared with me')
    await expect(sharedRow).toBeVisible({ timeout: 15_000 })
    await sharedRow.dblclick()
    await expect(page).toHaveURL(/__shared_with_me__/)

    await page.locator('aside').locator(':text-is("Share")').first().click()
    await page.waitForURL(/\/share/, { timeout: 15_000 })
    await expect(page.getByTestId('share-hub-unread-badge')).toHaveCount(0)
  })
})
