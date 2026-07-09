import { test, expect } from '@playwright/test'
import { randomEmail, randomPassword, createUser } from './helpers/auth'
import fs from 'fs'
import path from 'path'

const imageFixture = path.join(__dirname, 'fixtures', 'test-image.png')

// The capability response is intercepted at the network level instead of
// flipping the real admin setting, so the run stays free of cross-spec
// state (the admin PUT → capabilities chain is covered by the server's
// integration tests). Everything downstream is real: the browser encrypts
// with AEGIS-256 through WASM, the server stores the ciphertext, and the
// download decrypts it back.
test.describe('Default cipher from capabilities', () => {
  test('new uploads encrypt with the server-advertised cipher and round-trip', async ({
    page
  }) => {
    await page.route(/\/api\/capabilities$/, async (route) => {
      const response = await route.fetch()
      const caps = await response.json()
      caps.default_cipher = 'aegis256'
      await route.fulfill({ response, body: JSON.stringify(caps) })
    })

    // The capabilities store applies `default_cipher` when the fetch lands,
    // so uploading before that response is delivered would still use the
    // boot fallback. Anchor on the mocked response before touching files.
    const capsDelivered = page.waitForResponse(/\/api\/capabilities$/)
    await createUser(page, randomEmail(), randomPassword())
    await (await capsDelivered).finished()

    const created = page.waitForResponse(
      (r) => /\/api\/storage$/.test(r.url()) && r.request().method() === 'POST'
    )
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)

    const file = await (await created).json()
    expect(file.cipher).toBe('aegis256')

    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
    await expect(page.getByTestId('file-row-test-image.png')).toBeVisible()

    await page.getByTestId('file-row-test-image.png').locator('[name="actions-dropdown"]').click()
    const [download] = await Promise.all([
      page.waitForEvent('download'),
      page.locator('[name="download"]').first().click()
    ])

    expect(download.suggestedFilename()).toBe('test-image.png')
    const downloaded = fs.readFileSync(await download.path())
    expect(downloaded.equals(fs.readFileSync(imageFixture))).toBe(true)

    await page.unrouteAll({ behavior: 'ignoreErrors' })
  })
})
