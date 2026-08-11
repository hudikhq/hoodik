import { test, expect } from '@playwright/test'
import {
  randomEmail,
  randomPassword,
  createUser,
  createUserWithTwoFactor,
  loginAsUser,
  loginWithTwoFactor,
  loginWithPrivateKey,
  logout,
} from './helpers/auth'

test.describe('Registration', () => {
  test('can register a new user (skip 2FA) and land on the file browser', async ({ page }) => {
    const email = randomEmail()
    const password = randomPassword()
    const { privateKey } = await createUser(page, email, password)

    await expect(page).toHaveURL(/\/$/)
    // v2 accounts back up a curve recovery bundle, not an RSA PEM.
    expect(privateKey).toContain('ed:')
    expect(privateKey).toContain('x:')
    expect(privateKey).toContain('BEGIN PRIVATE KEY')
  })

  test('can register a new user with 2FA enabled', async ({ page }) => {
    const email = randomEmail()
    const password = randomPassword()
    const { secret } = await createUserWithTwoFactor(page, email, password)

    expect(secret).toBeTruthy()
    await expect(page).toHaveURL(/\/$/)
  })
})

test.describe('Login', () => {
  test('can log in with email and password', async ({ page }) => {
    const email = randomEmail()
    const password = randomPassword()
    // createUser registers + logs in; log out first so we can test login
    await createUser(page, email, password)
    await logout(page)

    await loginAsUser(page, email, password)
    await expect(page).toHaveURL(/\/$/)
  })

  test('can log in with 2FA OTP token', async ({ page }) => {
    const email = randomEmail()
    const password = randomPassword()
    const { secret } = await createUserWithTwoFactor(page, email, password)
    await logout(page)

    await loginWithTwoFactor(page, email, password, secret)
    await expect(page).toHaveURL(/\/$/)
  })

  test('can log in with private key', async ({ page }) => {
    const email = randomEmail()
    const password = randomPassword()
    const { privateKey } = await createUser(page, email, password)
    await logout(page)

    await loginWithPrivateKey(page, privateKey)
    await expect(page).toHaveURL(/\/$/)
  })
})

test.describe('Password manager hints', () => {
  test('login fields are annotated with autocomplete', async ({ page }) => {
    await page.goto('/auth/login')

    await expect(page.locator('#email')).toHaveAttribute('autocomplete', 'username')
    await expect(page.locator('#password')).toHaveAttribute('autocomplete', 'current-password')

    // The code field lives on the second step now, so it is not on this screen
    // at all until an account with 2FA gets that far.
    await expect(page.locator('#token')).toHaveCount(0)
  })

  test('registration fields are annotated with autocomplete', async ({ page }) => {
    await page.goto('/auth/register')

    await expect(page.locator('#email')).toHaveAttribute('autocomplete', 'username')
    await expect(page.locator('#password')).toHaveAttribute('autocomplete', 'new-password')
    await expect(page.locator('#confirm_password')).toHaveAttribute('autocomplete', 'new-password')
  })
})

test.describe('Logout', () => {
  test('logout redirects to login page', async ({ page }) => {
    const email = randomEmail()
    const password = randomPassword()
    await createUser(page, email, password)

    await logout(page)
    await expect(page).toHaveURL(/auth\/login/)
  })
})

test.describe('Theme', () => {
  test('login renders crimson links and submit button in the default dark theme', async ({
    page
  }) => {
    await page.goto('/auth/login')

    // dark:text-primary-100 is the crimson step that clears AA as text on the
    // dark card; the deeper fill steps measure ~2.6:1 and are not for text.
    const link = page.getByRole('link', { name: 'Create an Account' })
    await expect(link).toBeVisible()
    await expect(link).toHaveCSS('color', 'rgb(226, 103, 123)')

    await expect(page.getByRole('button', { name: 'Login', exact: true })).toHaveCSS(
      'background-color',
      'rgb(166, 52, 70)'
    )
  })

  test('the navbar toggle switches to light mode and the choice persists', async ({ page }) => {
    const email = randomEmail()
    const password = randomPassword()
    await createUser(page, email, password)

    await expect(page.locator('html')).toHaveClass(/dark/)

    await page.getByTestId('theme-toggle').click()
    await expect(page.locator('html')).not.toHaveClass(/dark/)
    expect(await page.evaluate(() => localStorage.getItem('lightMode'))).toBe('1')

    await page.getByTestId('theme-toggle').click()
    await expect(page.locator('html')).toHaveClass(/dark/)
    expect(await page.evaluate(() => localStorage.getItem('lightMode'))).toBe('0')
  })
})

test.describe('Lock screen', () => {
  test('forgetting a locked account requires confirmation', async ({ page }) => {
    const email = randomEmail()
    const password = randomPassword()
    await createUser(page, email, password)

    // First visit to the lock route without a stored pin walks through setup
    await page.locator('nav a[href*="/auth/pin/lock"]').click()
    await page.waitForURL('**/auth/pin/setup-lock-screen')
    await page.locator('#password').fill('1234')
    await page.locator('#confirm_password').fill('1234')
    await page.getByRole('button', { name: 'Encrypt and store' }).click()
    await page.waitForURL('**/')

    // A fresh load with a stored pin lands on the lock screen
    await page.goto('/auth/pin/lock')
    await page.getByRole('button', { name: 'Forget account' }).click()

    // Cancelling keeps the stored key and stays on the lock screen
    await expect(page.getByText('Forget this account?')).toBeVisible()
    await page.getByRole('button', { name: 'Cancel' }).click()
    await expect(page.getByText('Forget this account?')).not.toBeVisible()
    await expect(page).toHaveURL(/auth\/pin\/lock/)

    // Confirming forgets the account and returns to login
    await page.getByRole('button', { name: 'Forget account' }).click()
    await page.getByRole('button', { name: 'Yes, forget it' }).click()
    await page.waitForURL('**/auth/login')
  })
})
