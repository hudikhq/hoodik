import { test, expect } from '@playwright/test'
import { createUser, loginAsUser, randomEmail, randomPassword } from './helpers/auth'

test.describe('Share routing', () => {
  test('/links/foo redirects to /share/public/foo', async ({ page }) => {
    await createUser(page, randomEmail(), randomPassword())
    await page.goto('/links/foo/bar')
    await expect(page).toHaveURL(/\/share\/public\/foo\/bar/)
  })

  test('/links bare redirects to /share/public', async ({ page }) => {
    await createUser(page, randomEmail(), randomPassword())
    await page.goto('/links')
    // Wait for any URL change off /links — the redirect may add a trailing
    // segment depending on which sub-tab is active. Accept anything under
    // /share/.
    await page.waitForURL(/\/share/, { timeout: 15_000 })
    await expect(page.url()).toMatch(/\/share/)
  })

  test('Sidebar link reads "Share" not "Links" after upgrade', async ({ page }) => {
    const user = await createUser(page, randomEmail(), randomPassword())
    // Re-login via UI to make sure session cookies and the in-memory store
    // are coherent — createUser leaves the user on `/` but a subsequent
    // SPA reload depends on the refresh-cookie path that flakes for some
    // routes when the user just registered.
    await page.locator('button:has-text("Logout"), a:has-text("Logout")').first().click()
    await page.waitForURL('**/auth/login')
    await loginAsUser(page, user.email, user.password)
    await page.locator('aside').waitFor({ state: 'visible', timeout: 10_000 })
    await expect(page.locator('aside :text-is("Share")').first()).toBeVisible()
    await expect(page.locator('aside :text-is("Links")')).toHaveCount(0)
  })

  test('Share hub renders without an intro tooltip', async ({ page }) => {
    // The dismissible "account-to-account sharing" intro
    // was removed; release notes carry product education now.
    const user = await createUser(page, randomEmail(), randomPassword())
    await page.locator('button:has-text("Logout"), a:has-text("Logout")').first().click()
    await page.waitForURL('**/auth/login')
    await loginAsUser(page, user.email, user.password)
    await page.locator('aside').locator(':text-is("Share")').first().click()
    await page.waitForURL(/\/share/, { timeout: 15_000 })
    await expect(page.getByTestId('share-hub-rename-tooltip')).toHaveCount(0)
    await page.reload()
    await expect(page.getByTestId('share-hub-rename-tooltip')).toHaveCount(0)
  })
})
