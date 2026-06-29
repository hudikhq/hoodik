import { test, expect } from '@playwright/test'
import path from 'path'

import { createUser, loginAsUser, logout, randomEmail, randomPassword } from './helpers/auth'
import { discoverRecipient, openShareDialogFor } from './helpers/shares'

const imageFixture = path.join(__dirname, 'fixtures', 'test-image.png')

test.describe('Share hub navigation', () => {
  test('sidebar Share entry lands on /share/public with the three-tab control visible', async ({ page }) => {
    const user = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    await loginAsUser(page, user.email, user.password)

    await page.locator('aside').waitFor({ state: 'visible', timeout: 10_000 })
    await page.locator('aside').locator(':text-is("Share")').first().click()
    await page.waitForURL(/\/share\/public(?:\/|$|\?)/, { timeout: 15_000 })

    const tabs = page.getByTestId('share-hub-subtabs')
    await expect(tabs).toBeVisible({ timeout: 10_000 })
    await expect(page.getByTestId('share-hub-tab-public')).toBeVisible()
    await expect(page.getByTestId('share-hub-tab-activity')).toBeVisible()
    await expect(page.getByTestId('share-hub-tab-groups')).toBeVisible()
  })

  test('/share/audit redirects to /share/activity', async ({ page }) => {
    const user = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    // Remember Me persists the encrypted private key to local storage so
    // a subsequent `page.goto(...)` reload can re-derive in-memory state
    // from the refreshed session. Without it, /auth/refresh succeeds but
    // the absent private key throws and the SPA bounces to /auth/login.
    await loginAsUser(page, user.email, user.password, { remember: true })
    await page.locator('aside').waitFor({ state: 'visible', timeout: 10_000 })

    await page.goto('/share/audit')
    await page.waitForURL(/\/share\/activity/, { timeout: 15_000 })
    await expect(page).toHaveURL(/\/share\/activity/)
    await expect(page.getByTestId('share-hub-audit')).toBeVisible({ timeout: 15_000 })
  })

  test('/share/mine redirects to /share/public', async ({ page }) => {
    const user = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    await loginAsUser(page, user.email, user.password, { remember: true })
    await page.locator('aside').waitFor({ state: 'visible', timeout: 10_000 })

    await page.goto('/share/mine')
    await page.waitForURL(/\/share\/public/, { timeout: 15_000 })
    await expect(page).toHaveURL(/\/share\/public/)
  })

  test('/share/with-me redirects into the virtual folder under /files', async ({ page }) => {
    // The redirect needs a real incoming share so the synthetic root entry
    // is mounted in the file browser, which is the canonical landing pad.
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

    await loginAsUser(page, bob.email, bob.password, { remember: true })
    await page.locator('aside').waitFor({ state: 'visible', timeout: 10_000 })
    await page.goto('/share/with-me')
    await page.waitForURL(/__shared_with_me__/, { timeout: 15_000 })
    await expect(page).toHaveURL(/__shared_with_me__/)
    // The recipient is inside the synthetic folder, so the shared row is
    // already visible.
    await expect(page.getByTestId('file-row-test-image.png')).toBeVisible({ timeout: 15_000 })
  })
})
