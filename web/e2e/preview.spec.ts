import { test, expect } from '@playwright/test'
import { randomEmail, randomPassword, createUser } from './helpers/auth'
import path from 'path'
import fs from 'fs'

const imageFixture = path.join(__dirname, 'fixtures', 'test-image.png')
const imageFixture2 = path.join(__dirname, 'fixtures', 'test-image2.png')
const pdfFixture = path.join(__dirname, 'fixtures', 'test.pdf')
const videoFixture = path.join(__dirname, 'fixtures', 'test-video.mp4')

async function setup(page: Parameters<typeof createUser>[0]) {
  const email = randomEmail()
  const password = randomPassword()
  await createUser(page, email, password)
}

async function uploadAndWait(page: Parameters<typeof createUser>[0], fixturePath: string) {
  const filename = path.basename(fixturePath)
  await page.setInputFiles('[name="upload-file-input"]', fixturePath)
  // Wait for the file row to appear — reliable for both fast and slow uploads
  await page.getByTestId(`file-row-${filename}`).first().waitFor({ state: 'visible', timeout: 30_000 })
}

test.describe('Image preview', () => {
  test('opens image preview on double-click and shows the full image', async ({ page }) => {
    await setup(page)
    await uploadAndWait(page, imageFixture)

    await page.getByTestId('file-row-test-image.png').dblclick()
    await expect(page).toHaveURL(/file-preview|\/p\//)

    await expect(page.locator('img[name="original"]')).toBeVisible()
    await expect(page.locator('img[name="original"]')).toHaveAttribute('alt', 'test-image.png')
  })

  test('can navigate to next/previous file with arrow buttons', async ({ page }) => {
    await setup(page)

    // Upload two images with different names so navigation works
    await uploadAndWait(page, imageFixture)
    await uploadAndWait(page, imageFixture2)

    await page.getByTestId('file-row-test-image.png').dblclick()
    await expect(page.locator('img[name="original"]')).toBeVisible()

    // Check the counter shows "X / 2"
    const counter = page.locator('text=/\\d+ \\/ \\d+/')
    await expect(counter).toBeVisible()
    const initial = await counter.textContent()

    // Click the right arrow (rendered as a link with title "Next image")
    await page.locator('[title="Next image"]').click()
    await expect(counter).not.toHaveText(initial!)
    const next = await counter.textContent()
    expect(next).not.toBe(initial)
  })

  test('Escape key closes the preview', async ({ page }) => {
    await setup(page)
    await uploadAndWait(page, imageFixture)

    await page.getByTestId('file-row-test-image.png').dblclick()
    await expect(page.locator('img[name="original"]')).toBeVisible()

    await page.keyboard.press('Escape')
    await expect(page).not.toHaveURL(/file-preview|\/p\//)
    await expect(page.locator('img[name="original"]')).not.toBeVisible()
  })

  test('preview-close button closes the preview', async ({ page }) => {
    await setup(page)
    await uploadAndWait(page, imageFixture)

    await page.getByTestId('file-row-test-image.png').dblclick()
    await page.locator('[name="preview-close"]').click()
    await expect(page).not.toHaveURL(/file-preview|\/p\//)
  })

  test('can download from the preview bar', async ({ page }) => {
    await setup(page)
    await uploadAndWait(page, imageFixture)

    await page.getByTestId('file-row-test-image.png').dblclick()
    await expect(page.locator('img[name="original"]')).toBeVisible()

    const [download] = await Promise.all([
      page.waitForEvent('download'),
      page.locator('[name="preview-download"]').click(),
    ])
    expect(download.suggestedFilename()).toBe('test-image.png')
  })

  test('can open the details panel from preview', async ({ page }) => {
    await setup(page)
    await uploadAndWait(page, imageFixture)

    await page.getByTestId('file-row-test-image.png').dblclick()
    await page.locator('[name="preview-details"]').click()

    // Details panel shows the MIME type
    await expect(page.locator('text=image/png')).toBeVisible()
  })
})

test.describe('PDF preview', () => {
  test('opens PDF viewer for a PDF file', async ({ page }) => {
    await setup(page)
    await uploadAndWait(page, pdfFixture)

    await page.getByTestId('file-row-test.pdf').dblclick()
    await expect(page).toHaveURL(/file-preview|\/p\//)

    // PDF viewer renders an iframe with a blob: URL
    await expect(page.locator('iframe').first()).toBeVisible({ timeout: 15_000 })
  })
})

test.describe('Video preview', () => {
  test('opens video player for a video file', async ({ page }) => {
    await setup(page)

    if (!fs.existsSync(videoFixture)) {
      test.skip()
      return
    }

    await uploadAndWait(page, videoFixture)

    await page.getByTestId('file-row-test-video.mp4').dblclick()
    await expect(page).toHaveURL(/file-preview|\/p\//)

    // The <video> element should appear (MSE or blob URL mode)
    await expect(page.locator('video')).toBeVisible({ timeout: 20_000 })
  })
})
