import { test, expect } from '@playwright/test'
import { randomEmail, randomPassword, createUser } from './helpers/auth'
import path from 'path'
import fs from 'fs'
import os from 'os'

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

    // Open actions dropdown and click "Sharing", then switch to the
    // Link tab inside the unified modal.
    await page.getByTestId('file-row-test-image.png').locator('[name="actions-dropdown"]').click()
    await page.locator('[name="sharing"]').first().click()
    await page.getByTestId('sharing-modal-tab-link').click()

    // Create the link
    await page.getByTestId('sharing-link-create').click()

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

test.describe('Public link download is zero-knowledge', () => {
  test('a multi-chunk download streams ciphertext and never carries the link key', async ({
    page,
    browser
  }) => {
    const email = randomEmail()
    const password = randomPassword()
    await createUser(page, email, password)

    // Larger than one 4 MB chunk so the chunked download path runs — every
    // other web e2e uses a single-chunk file. The marker fill lets us prove the
    // streamed bytes are ciphertext and not the plaintext we uploaded.
    const marker = Buffer.from('HOODIK-PLAINTEXT-MARKER')
    const bigFile = path.join(os.tmpdir(), `hoodik-e2e-${Date.now()}.bin`)
    fs.writeFileSync(bigFile, Buffer.alloc(5 * 1024 * 1024, marker))

    try {
      const filename = path.basename(bigFile)
      await page.setInputFiles('[name="upload-file-input"]', bigFile)
      await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 60_000 })
      await expect(page.getByTestId(`file-row-${filename}`)).toBeVisible()

      await page.getByTestId(`file-row-${filename}`).locator('[name="actions-dropdown"]').click()
      await page.locator('[name="sharing"]').first().click()
      await page.getByTestId('sharing-modal-tab-link').click()
      await page.getByTestId('sharing-link-create').click()

      const linkUrl = await page.locator('input[name="link"]').inputValue()
      const linkKeyHex = new URL(linkUrl).hash.replace('#', '')
      expect(linkKeyHex).toBeTruthy()

      const anonContext = await browser.newContext()
      const anonPage = await anonContext.newPage()

      const chunks: { url: string; postData: string | null; body: Promise<Buffer> }[] = []
      anonPage.on('response', (resp) => {
        const url = resp.url()
        if (/\/api\/links\/[0-9a-f-]+\?chunk=/i.test(url)) {
          chunks.push({ url, postData: resp.request().postData(), body: resp.body() })
        }
      })

      await anonPage.goto(linkUrl)
      await expect(anonPage.locator('[name="preview-download"]')).toBeVisible({ timeout: 30_000 })
      await Promise.all([
        anonPage.waitForEvent('download'),
        anonPage.locator('[name="preview-download"]').click()
      ])
      await Promise.all(chunks.map((c) => c.body))

      // More than one chunk was fetched, each request carried nothing but the
      // chunk index, and no request leaked the link key.
      expect(chunks.length).toBeGreaterThan(1)
      for (const chunk of chunks) {
        expect([...new URL(chunk.url).searchParams.keys()]).toEqual(['chunk'])
        expect(chunk.url).not.toContain(linkKeyHex)
        expect(chunk.postData ?? '').not.toContain(linkKeyHex)
        // The server streams stored ciphertext; the plaintext marker must be absent.
        expect((await chunk.body).includes(marker)).toBe(false)
      }

      await anonContext.close()
    } finally {
      fs.rmSync(bigFile, { force: true })
    }
  })
})
