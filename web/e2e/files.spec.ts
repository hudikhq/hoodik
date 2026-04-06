import { test, expect } from '@playwright/test'
import { randomEmail, randomPassword, createUser } from './helpers/auth'
import path from 'path'

const imageFixture = path.join(__dirname, 'fixtures', 'test-image.png')

async function setup(page: Parameters<typeof createUser>[0]) {
  const email = randomEmail()
  const password = randomPassword()
  // createUser registers and leaves the user fully logged in at '/'
  await createUser(page, email, password)
}

test.describe('Directories', () => {
  test('can create a directory and navigate into it', async ({ page }) => {
    await setup(page)

    // Create directory
    await page.locator('[name="create-dir"]').click()
    await page.locator('#name').fill('My_Test_Dir')
    await page.getByRole('button', { name: 'Create', exact: true }).click()

    // Directory appears in list
    await expect(page.getByTestId('file-row-My_Test_Dir')).toBeVisible()

    // Double-click to navigate inside (URL becomes a UUID path, not the root '/')
    await page.getByTestId('file-row-My_Test_Dir').dblclick()
    await expect(page).not.toHaveURL(/^http:\/\/localhost:\d+\/$/)
    await expect(page).toHaveURL(/[0-9a-f-]{36}/)

    // Navigate back via breadcrumb
    await page.getByLabel('Breadcrumb').getByRole('link', { name: 'My Files' }).click()
    await expect(page.getByTestId('file-row-My_Test_Dir')).toBeVisible()
  })
})

test.describe('Upload', () => {
  test('can upload an image file and see the thumbnail', async ({ page }) => {
    await setup(page)

    await page.setInputFiles('[name="upload-file-input"]', imageFixture)

    // Wait for the active upload to finish
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    // File row with thumbnail appears
    await expect(page.getByTestId('file-row-test-image.png')).toBeVisible()
    await expect(page.locator('img[name="thumbnail"][alt="test-image.png"]')).toBeVisible()
  })
})

test.describe('Download', () => {
  test('can download an uploaded file', async ({ page }) => {
    await setup(page)

    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
    await expect(page.getByTestId('file-row-test-image.png')).toBeVisible()

    // Open the actions dropdown for the file
    await page.getByTestId('file-row-test-image.png').locator('[name="actions-dropdown"]').click()

    // Start download and capture the file
    const [download] = await Promise.all([
      page.waitForEvent('download'),
      page.locator('[name="download"]').first().click(),
    ])

    expect(download.suggestedFilename()).toBe('test-image.png')
  })
})

test.describe('Rename', () => {
  test('can rename a file', async ({ page }) => {
    await setup(page)

    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    // Open actions, click rename
    await page.getByTestId('file-row-test-image.png').locator('[name="actions-dropdown"]').click()
    await page.locator('[name="rename"]').first().click()

    // Fill in the rename input and confirm
    const nameInput = page.getByPlaceholder('new name')
    await nameInput.fill('renamed-image.png')
    await page.getByRole('button', { name: 'Rename' }).click()

    await expect(page.getByTestId('file-row-renamed-image.png')).toBeVisible()
    await expect(page.getByTestId('file-row-test-image.png')).not.toBeVisible()
  })
})

test.describe('Delete', () => {
  test('can delete a file', async ({ page }) => {
    await setup(page)

    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    // Open actions, click delete
    await page.getByTestId('file-row-test-image.png').locator('[name="actions-dropdown"]').click()
    await page.locator('[name="delete"]').first().click()

    // Confirm deletion if a modal appears
    const confirmBtn = page.getByRole('button', { name: 'Delete' })
    if (await confirmBtn.isVisible()) {
      await confirmBtn.click()
    }

    await expect(page.getByTestId('file-row-test-image.png')).not.toBeVisible()
  })
})
