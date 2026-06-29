import { test, expect } from '@playwright/test'
import { authenticator } from 'otplib'
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

test.describe('Change password', () => {
  test('with no TFA, change password via current password succeeds', async ({ page }) => {
    const email = randomEmail()
    const password = randomPassword()
    await createUser(page, email, password)

    await openChangePasswordViaSidebar(page)
    await page.locator('#current_password').fill(password)

    const newPassword = randomPassword()
    await page.locator('#password').fill(newPassword)

    const response = page.waitForResponse(
      (res) =>
        res.url().includes('/api/auth/account/change-password') &&
        res.request().method() === 'POST'
    )
    await page.getByRole('button', { name: 'Change password' }).click()
    expect((await response).status()).toBe(204)

    await logout(page)
    await loginAsUser(page, email, newPassword)
    await expect(page).toHaveURL(/\/$/)
  })

  // Regression test for https://github.com/hudikhq/hoodik/issues/164
  test('with TFA enabled, change password via private key + OTP succeeds', async ({ page }) => {
    const email = randomEmail()
    const password = randomPassword()
    const { privateKey, secret } = await createUserWithTwoFactor(page, email, password)

    await openChangePasswordViaSidebar(page)
    await page.locator('#use_private_key').check()
    await page.locator('#private_key').fill(privateKey)
    await page.locator('#token').fill(authenticator.generate(secret))

    const newPassword = randomPassword()
    await page.locator('#password').fill(newPassword)

    const response = page.waitForResponse(
      (res) =>
        res.url().includes('/api/auth/account/change-password') &&
        res.request().method() === 'POST'
    )
    await page.getByRole('button', { name: 'Change password' }).click()

    // The bug in #164 manifests as a 401 with body `invalid_otp_token`
    // because the frontend silently dropped the token before sending.
    // After the fix the backend returns 204.
    expect((await response).status()).toBe(204)

    await logout(page)
    await loginWithTwoFactor(page, email, newPassword, secret)
    await expect(page).toHaveURL(/\/$/)
  })
})
