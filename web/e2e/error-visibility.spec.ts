import { test, expect } from '@playwright/test'
import { randomEmail, randomPassword, createUser } from './helpers/auth'

/**
 * A failed request must always tell the user something. These drive real
 * failures through the app's error surfaces rather than trusting that a
 * catch block was wired up: a toast that never renders looks exactly like
 * a catch that swallows.
 */
test.describe('Error visibility', () => {
  test('a failed account preference toggle shows a toast', async ({ page }) => {
    await createUser(page, randomEmail(), randomPassword())
    await page.locator('aside').getByText('Account', { exact: true }).first().click()

    const toggle = page.getByTestId('account-share-notifications-toggle')
    await toggle.waitFor()

    await page.route('**/api/users/me', (route) =>
      route.fulfill({
        status: 500,
        contentType: 'application/json',
        body: JSON.stringify({ status: 500, message: 'internal' })
      })
    )

    await toggle.click()

    const toast = page.locator('.vue-notification')
    await expect(toast).toBeVisible()
    await expect(toast).toContainText(/request failed/i)
  })

  test('a failed folder creation shows an inline error in the dialog', async ({ page }) => {
    await createUser(page, randomEmail(), randomPassword())

    await page.route(
      (url) => url.pathname === '/api/storage',
      (route) =>
        route.request().method() === 'POST'
          ? route.fulfill({
              status: 500,
              contentType: 'application/json',
              body: JSON.stringify({ status: 500, message: 'internal' })
            })
          : route.fallback()
    )

    await page.locator('[name="create-dir"]').click()
    await page.locator('#name').fill('Some_Folder')
    await page.getByRole('button', { name: 'Create', exact: true }).click()

    const dialog = page.getByRole('dialog', { name: 'Create a folder' })
    await expect(dialog).toBeVisible()
    await expect(dialog.getByRole('alert')).toBeVisible()
  })
})
