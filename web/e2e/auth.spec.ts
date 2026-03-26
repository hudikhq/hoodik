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
    expect(privateKey).toContain('BEGIN RSA PRIVATE KEY')
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

test.describe('Logout', () => {
  test('logout redirects to login page', async ({ page }) => {
    const email = randomEmail()
    const password = randomPassword()
    await createUser(page, email, password)

    await logout(page)
    await expect(page).toHaveURL(/auth\/login/)
  })
})
