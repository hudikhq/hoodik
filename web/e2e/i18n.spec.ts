import { test, expect } from '@playwright/test'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { randomEmail, randomPassword, createUser } from './helpers/auth'

const messages = (locale: string) =>
  JSON.parse(readFileSync(join(__dirname, `../src/locales/${locale}.json`), 'utf-8'))

test.describe('Localization', () => {
  test.describe('browser locale detection', () => {
    test.use({ locale: 'fr-FR' })

    test('a French browser gets the French UI without any setup', async ({ page }) => {
      await page.goto('/auth/login')

      await expect(page.locator('html')).toHaveAttribute('lang', 'fr')
      await expect(page.getByText(messages('fr').auth.login.title)).toBeVisible()
    })
  })

  test('unsupported browser locales fall back to English', async ({ browser }) => {
    const context = await browser.newContext({ locale: 'pt-BR' })
    const page = await context.newPage()
    await page.goto('/auth/login')

    await expect(page.locator('html')).toHaveAttribute('lang', 'en')
    await context.close()
  })

  test('switching language in account settings applies and persists', async ({ page }) => {
    const email = randomEmail()
    const password = randomPassword()
    await createUser(page, email, password)

    // Reach the account view through the in-app sidebar rather than a hard
    // page.goto — a full reload would drop the in-memory decrypted key and
    // bounce the authenticated view back to login.
    await page.locator('aside').getByText('Account', { exact: true }).first().click()
    await page.waitForURL(/\/account(\/|$|\?)/)

    const select = page.getByTestId('account-language-select')
    await expect(select).toBeVisible()

    await select.selectOption('hr')

    await expect(page.locator('html')).toHaveAttribute('lang', 'hr')
    await expect(page.getByText(messages('hr').account.preferences).first()).toBeVisible()

    // A reload drops the session and lands on login, which must now render in
    // Croatian — proving the choice persisted to localStorage, not just to the
    // live i18n instance.
    await page.reload()

    await expect(page.locator('html')).toHaveAttribute('lang', 'hr')
    await expect(page.getByText(messages('hr').auth.login.title)).toBeVisible()
  })
})
