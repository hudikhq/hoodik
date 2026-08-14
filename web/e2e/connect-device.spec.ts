import { test, expect } from '@playwright/test'
import { randomEmail, randomPassword, createUser, loginAsUser, logout } from './helpers/auth'

/**
 * The QR code is what a user points a phone at to set the mobile app up. Its
 * payload is covered by tests/account/connect-device-card.test.ts; these check
 * that both surfaces carrying it reach a signed-in user.
 */
test.describe('Connect a device', () => {
  test('a new account is offered the app, once', async ({ page }) => {
    const email = randomEmail()
    const password = randomPassword()

    // createUser dismisses the prompt itself — every other spec in the suite
    // depends on that, so assert it was there to dismiss.
    await page.goto('/auth/register')
    await page.locator('#email').fill(email)
    await page.locator('#password').fill(password)
    await page.locator('#confirm_password').fill(password)
    await page.getByRole('button', { name: 'Next' }).click()
    await page.waitForURL('**/register/key')
    await page.locator('#i_have_stored_my_private_key').check()
    await page.getByRole('button', { name: 'Next' }).click()
    await page.waitForURL('**/register/two-factor')
    await page.getByRole('button', { name: 'Skip' }).click()
    await page.waitForURL('**/', { waitUntil: 'load' })

    const prompt = page.getByTestId('connect-prompt-qr')
    await expect(prompt).toBeVisible()

    await page.getByRole('button', { name: 'Done', exact: true }).click()
    await expect(prompt).toBeHidden()

    // Dismissal sticks across a re-login, or it stops being a welcome and
    // becomes something the user has to swat away every morning.
    await logout(page)
    await loginAsUser(page, email, password)
    await expect(prompt).toBeHidden()
  })

  test('the account page keeps the code for later', async ({ page }) => {
    await createUser(page, randomEmail(), randomPassword())

    await page.locator('aside').getByText('Account', { exact: true }).first().click()

    const qr = page.getByTestId('account-connect-qr')
    await expect(qr).toBeVisible()
    await expect(qr.locator('svg')).toBeVisible()
  })
})

/**
 * A QR code is unusable on the screen you are already holding, so a phone gets
 * a button that opens the app instead. Playwright's desktop agent never sees
 * that branch, hence its own context.
 */
test.describe('Connect a device, on a phone', () => {
  test.use({
    userAgent:
      'Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1',
    viewport: { width: 390, height: 844 }
  })

  test('is offered the app rather than a code it cannot scan', async ({ page }) => {
    // Registered inline rather than through createUser, which dismisses the
    // prompt this test is here to look at.
    await page.goto('/auth/register')
    await page.locator('#email').fill(randomEmail())
    await page.locator('#password').fill(randomPassword())
    await page.locator('#confirm_password').fill(randomPassword())
    await page.locator('#confirm_password').fill(await page.locator('#password').inputValue())
    await page.getByRole('button', { name: 'Next' }).click()
    await page.waitForURL('**/register/key')
    await page.locator('#i_have_stored_my_private_key').check()
    await page.getByRole('button', { name: 'Next' }).click()
    await page.waitForURL('**/register/two-factor')
    await page.getByRole('button', { name: 'Skip' }).click()
    await page.waitForURL('**/', { waitUntil: 'load' })

    await expect(page.getByTestId('connect-prompt-open-app')).toBeVisible()
    await expect(page.getByTestId('connect-prompt-qr')).toHaveCount(0)
  })
})
