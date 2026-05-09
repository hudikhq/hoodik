import { test, expect } from '@playwright/test'

// Verify the SPA's reaction to the public `allow_register=false` signal by
// intercepting the new `GET /api/auth/register/status` endpoint at the
// network level rather than mutating real server state. That keeps the test
// self-contained: no admin role required, no state cleanup, no ordering
// dependency on other specs in the run.
test.describe('Registration disabled (allow_register=false from server)', () => {
  test.beforeEach(async ({ page }) => {
    await page.route(/\/api\/auth\/register\/status$/, async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ allow_register: false })
      })
    })
  })

  test('login page hides the "Create an Account" link', async ({ page }) => {
    const statusReady = page.waitForResponse(/\/api\/auth\/register\/status$/)
    await page.goto('/auth/login')
    await statusReady
    await expect(page.getByRole('link', { name: /Recover your account/i })).toBeVisible()
    await expect(page.getByRole('link', { name: 'Create an Account' })).toHaveCount(0)
  })

  test('private-key login page hides the "Create an Account" link', async ({ page }) => {
    const statusReady = page.waitForResponse(/\/api\/auth\/register\/status$/)
    await page.goto('/auth/login/private-key')
    await statusReady
    await expect(page.locator('#privateKey')).toBeVisible()
    await expect(page.getByRole('link', { name: 'Create an Account' })).toHaveCount(0)
  })

  test('register page replaces the form with the disabled message (no invitation)', async ({ page }) => {
    const statusReady = page.waitForResponse(/\/api\/auth\/register\/status$/)
    await page.goto('/auth/register')
    await statusReady
    await expect(page.getByTestId('registration-disabled')).toBeVisible()
    await expect(page.locator('#email')).toHaveCount(0)
    await expect(page.getByRole('link', { name: 'Back to login' })).toBeVisible()
  })

  test('register page still shows the form when an invitation_id is present', async ({ page }) => {
    // Server-side validation enforces invitation correctness; the SPA only
    // needs to keep the form available so invited users can submit.
    await page.goto('/auth/register?invitation_id=00000000-0000-0000-0000-000000000000')
    await expect(page.getByTestId('registration-disabled')).toHaveCount(0)
    await expect(page.locator('#email')).toBeVisible()
    await expect(page.locator('#password')).toBeVisible()
  })
})
