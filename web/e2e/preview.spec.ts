import { test, expect } from '@playwright/test'
import { randomEmail, randomPassword, createUser } from './helpers/auth'
import path from 'path'
import fs from 'fs'

const imageFixture = path.join(__dirname, 'fixtures', 'test-image.png')
const imageFixture2 = path.join(__dirname, 'fixtures', 'test-image2.png')
const pdfFixture = path.join(__dirname, 'fixtures', 'test.pdf')
const videoFixture = path.join(__dirname, 'fixtures', 'test-video.mp4')
const textFixture = path.join(__dirname, 'fixtures', 'sample-textlike-file.tf')

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

test.describe('Text preview', () => {
  test('shows a text file without a text mime as numbered monospaced text', async ({ page }) => {
    await setup(page)
    await uploadAndWait(page, textFixture)

    await page.getByTestId('file-row-sample-textlike-file.tf').dblclick()
    await expect(page).toHaveURL(/file-preview|\/p\//)

    const viewer = page.getByTestId('preview-text')
    await expect(viewer).toBeVisible({ timeout: 15_000 })
    const lines = viewer.locator('.text-content')
    await expect(lines).toHaveCount(7)
    // toHaveText collapses whitespace, so compare the raw text to check indentation
    await expect(lines.nth(3)).toHaveJSProperty('textContent', '    apple  = "red"')
    await expect(lines.nth(4)).toHaveJSProperty('textContent', '\tbanana = "yellow"')
    await expect(viewer).toContainText('7 lines')
    await expect(page.getByTestId('preview-text-truncated')).toHaveCount(0)

    const wrap = page.locator('[name="preview-text-wrap"]')
    await expect(wrap).toHaveAttribute('aria-pressed', 'false')
    await wrap.click()
    await expect(wrap).toHaveAttribute('aria-pressed', 'true')
    await expect(viewer.locator('.text-body')).toHaveClass(/is-wrapped/)
  })

  test('arrow keys scroll the focused text instead of switching files', async ({ page }) => {
    await setup(page)
    await uploadAndWait(page, textFixture)
    await uploadAndWait(page, imageFixture)

    await page.getByTestId('file-row-sample-textlike-file.tf').dblclick()
    await expect(page.getByTestId('preview-text')).toBeVisible({ timeout: 15_000 })

    const counter = page.getByTestId('preview-counter')
    await expect(counter).toHaveText(/[12] \/ 2/)
    const before = (await counter.textContent())!
    const towardsOther = before.trim().startsWith('1') ? 'ArrowRight' : 'ArrowLeft'

    await page.getByTestId('preview-text-pane').click()
    await page.keyboard.press(towardsOther)
    // Nothing to wait on when nothing happens; give a navigation time to show up.
    await page.waitForTimeout(500)
    await expect(counter).toHaveText(before)
    await expect(page.getByTestId('preview-text')).toBeVisible()

    // Focus outside the pane: the same key switches to the other file again.
    await counter.click()
    await page.keyboard.press(towardsOther)
    await expect(counter).not.toHaveText(before)
  })
})
