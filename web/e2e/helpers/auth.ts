import type { Page } from '@playwright/test'
import { authenticator } from 'otplib'

export function randomEmail() {
  return `test+${Math.floor(Math.random() * 1e9)}@test.com`
}

export function randomPassword() {
  return `strong-password-${Math.floor(Math.random() * 1e9)}`
}

/**
 * Register a new user and skip 2FA.
 * Returns the captured private key along with the credentials used.
 */
export async function createUser(page: Page, email: string, password: string) {
  await page.goto('/auth/register')
  await page.locator('#email').fill(email)
  await page.locator('#password').fill(password)
  await page.locator('#confirm_password').fill(password)
  await page.getByRole('button', { name: 'Next' }).click()
  await page.waitForURL('**/register/key')

  const privateKey = await page.locator('#unencrypted_private_key').inputValue()
  await page.locator('#i_have_stored_my_private_key').check()
  await page.getByRole('button', { name: 'Next' }).click()
  await page.waitForURL('**/register/two-factor')
  await page.getByRole('button', { name: 'Skip' }).click()
  // Wait until the app has fully redirected to the file browser
  await page.waitForURL('**/', { waitUntil: 'load' })

  return { email, password, privateKey }
}

/**
 * Log out the currently authenticated user.
 * Waits for redirect to the login page.
 */
export async function logout(page: Page) {
  // Logout is rendered as a sidebar button (not an <a> link)
  await page.locator('button:has-text("Logout"), a:has-text("Logout")').first().click()
  await page.waitForURL('**/auth/login')
}

/**
 * Register a new user with 2FA enabled.
 * Returns the OTP secret and private key along with credentials.
 */
export async function createUserWithTwoFactor(page: Page, email: string, password: string) {
  await page.goto('/auth/register')
  await page.locator('#email').fill(email)
  await page.locator('#password').fill(password)
  await page.locator('#confirm_password').fill(password)
  await page.getByRole('button', { name: 'Next' }).click()
  await page.waitForURL('**/register/key')

  const privateKey = await page.locator('#unencrypted_private_key').inputValue()
  await page.locator('#i_have_stored_my_private_key').check()
  await page.getByRole('button', { name: 'Next' }).click()
  await page.waitForURL('**/register/two-factor')

  const secret = await page.locator('#secret').inputValue()
  const token = authenticator.generate(secret)
  await page.locator('#token').fill(token)
  await page.getByRole('button', { name: 'Register with Two Factor' }).click()

  return { email, password, privateKey, secret }
}

/**
 * Log in with email and password (no 2FA).
 * Waits for redirect to the files view.
 */
export async function loginAsUser(page: Page, email: string, password: string) {
  await page.goto('/auth/login')
  await page.locator('#email').fill(email)
  await page.locator('#password').fill(password)
  await page.getByRole('button', { name: 'Login' }).click()
  await page.waitForURL('**/')
}

/**
 * Log in with email, password, and a TOTP token (2FA enabled account).
 */
export async function loginWithTwoFactor(page: Page, email: string, password: string, secret: string) {
  await page.goto('/auth/login')
  await page.locator('#email').fill(email)
  await page.locator('#password').fill(password)
  await page.locator('#token').fill(authenticator.generate(secret))
  await page.getByRole('button', { name: 'Login' }).click()
  await page.waitForURL('**/')
}

/**
 * Log in using a PEM private key (alternative login method).
 */
export async function loginWithPrivateKey(page: Page, privateKey: string) {
  await page.goto('/auth/login')
  await page.getByRole('link', { name: 'Login With Private Key' }).click()
  await page.waitForURL('**/private-key')
  await page.locator('#privateKey').fill(privateKey)
  await page.getByRole('button', { name: 'Login' }).click()
  await page.waitForURL('**/')
}
