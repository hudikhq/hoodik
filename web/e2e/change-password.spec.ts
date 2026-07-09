import { test, expect } from '@playwright/test'
import {
  randomEmail,
  randomPassword,
  createUser,
  createUserWithTwoFactor,
  loginAsUser,
  loginWithTwoFactor,
  logout,
} from './helpers/auth'

/**
 * Walk from the authenticated file browser to the change-password screen
 * via the in-app sidebar + "My account" link. A full `page.goto` would
 * reload the SPA and drop the in-memory keypair, bouncing the user back
 * to /auth/login.
 */
async function openChangePasswordViaSidebar(page: import('@playwright/test').Page) {
  await page.locator('aside').getByText('Account', { exact: true }).first().click()
  await page.waitForURL(/\/account(\/|$|\?)/)
  await page.getByRole('link', { name: 'Change', exact: true }).click()
  await page.waitForURL(/\/account\/change-password/)
}

/**
 * v2 (Curve25519 + OPAQUE) accounts change their password through the PAKE
 * flow: the session authenticates the request and the in-memory keys re-seal
 * the private-key envelope under a KEK derived from the new password's
 * OPAQUE `export_key`. The legacy proof inputs are hidden, so the form only
 * needs the new password. All accounts created through registration are v2.
 */
test.describe('Change password', () => {
  test('v2 account changes its password through the PAKE flow', async ({ page }) => {
    const email = randomEmail()
    const password = randomPassword()
    await createUser(page, email, password)

    await openChangePasswordViaSidebar(page)

    const newPassword = randomPassword()
    await page.locator('#password').fill(newPassword)

    const finished = page.waitForResponse(
      (res) =>
        res.url().includes('/api/auth/pake/register/finish') &&
        res.request().method() === 'POST'
    )
    await page.getByRole('button', { name: 'Change password' }).click()
    expect((await finished).ok()).toBeTruthy()

    // The old password no longer works; the new one does.
    await logout(page)
    await loginAsUser(page, email, newPassword)
    await expect(page).toHaveURL(/\/$/)
  })

  test('a 2FA-enabled v2 account changes its password and logs back in with 2FA', async ({
    page,
  }) => {
    const email = randomEmail()
    const password = randomPassword()
    const { secret } = await createUserWithTwoFactor(page, email, password)

    await openChangePasswordViaSidebar(page)

    const newPassword = randomPassword()
    await page.locator('#password').fill(newPassword)

    const finished = page.waitForResponse(
      (res) =>
        res.url().includes('/api/auth/pake/register/finish') &&
        res.request().method() === 'POST'
    )
    await page.getByRole('button', { name: 'Change password' }).click()
    expect((await finished).ok()).toBeTruthy()

    await logout(page)
    await loginWithTwoFactor(page, email, newPassword, secret)
    await expect(page).toHaveURL(/\/$/)
  })
})
