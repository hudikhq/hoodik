import { test, expect } from '@playwright/test'
import { randomEmail, randomPassword, createUser } from './helpers/auth'
import path from 'path'

const imageFixture = path.join(__dirname, 'fixtures', 'test-image.png')

async function setup(page: Parameters<typeof createUser>[0]) {
  const email = randomEmail()
  const password = randomPassword()
  await createUser(page, email, password)
}

test.describe('Public links', () => {
  test('can create a public link for a file and view it without authentication', async ({ page, browser }) => {
    await setup(page)

    // Upload a file
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
    await expect(page.getByTestId('file-row-test-image.png')).toBeVisible()

    // Open actions dropdown and click "Public link"
    await page.getByTestId('file-row-test-image.png').locator('[name="actions-dropdown"]').click()
    await page.locator('[name="public-link"]').first().click()

    // Create the link
    await page.getByRole('button', { name: 'Create link' }).click()

    // Read the generated link URL
    const linkUrl = await page.locator('input[name="link"]').inputValue()
    expect(linkUrl).toBeTruthy()

    // Open the link in a fresh browser context (unauthenticated)
    const anonContext = await browser.newContext()
    const anonPage = await anonContext.newPage()
    await anonPage.goto(linkUrl)

    // The linked file's image should be visible without logging in
    await expect(anonPage.locator('img[name="original"]')).toBeVisible({ timeout: 30_000 })
    await expect(anonPage.locator('img[name="original"]')).toHaveAttribute('alt', 'test-image.png')

    await anonContext.close()
  })
})
