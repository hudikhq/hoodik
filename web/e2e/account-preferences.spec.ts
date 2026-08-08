import { test, expect } from '@playwright/test'
import { randomEmail, randomPassword, createUser, loginAsUser, logout } from './helpers/auth'

/**
 * The sharing-notification toggle is the app's only PATCH request. Its unit
 * test mocks the API client, so a wrong HTTP verb was invisible there: the
 * client sent a lowercase `patch`, which fetch does not normalize the way it
 * normalizes GET/POST/PUT/DELETE, and actix answered 404. Drive the real
 * request and assert the value survives a round trip.
 */
test.describe('Account preferences', () => {
  test('the sharing-notification toggle persists across a re-login', async ({ page }) => {
    const email = randomEmail()
    const password = randomPassword()
    await createUser(page, email, password)

    await page.locator('aside').getByText('Account', { exact: true }).first().click()

    const toggle = page.getByTestId('account-share-notifications-toggle')
    await toggle.waitFor()
    await expect(toggle).toBeChecked()

    const patch = page.waitForResponse(
      (res) => res.url().includes('/api/users/me') && res.request().method() === 'PATCH'
    )
    await toggle.click()
    expect((await patch).status()).toBe(200)

    await expect(toggle).not.toBeChecked()

    await logout(page)
    await loginAsUser(page, email, password)
    await page.locator('aside').getByText('Account', { exact: true }).first().click()

    await expect(page.getByTestId('account-share-notifications-toggle')).not.toBeChecked()
  })
})
