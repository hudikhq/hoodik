import { test, expect } from '@playwright/test'
import path from 'path'
import fs from 'fs'
import { randomEmail, randomPassword, createUser } from './helpers/auth'
import {
  createNoteFromBrowser,
  openRawMarkdown,
  typeRawMarkdown,
  saveViaButton
} from './helpers/notes'

/**
 * A2a/A2b — layout split between legacy (single-version) storage and
 * the new versioned-chunks storage. From the browser's point of view:
 *   - Uploaded regular files follow the legacy path: chunks live under
 *     `<file-id>/0`, `<file-id>/1`, etc.
 *   - Editable notes follow the versioned path: chunks live under
 *     `<file-id>/v{N}/0`, `v{N}/1`, etc.
 * Both must download correctly through the same GET endpoint.
 */

const imageFixture = path.join(__dirname, 'fixtures', 'test-image.png')

async function setup(page: Parameters<typeof createUser>[0]) {
  await createUser(page, randomEmail(), randomPassword())
}

test.describe('Legacy (non-editable) download path', () => {
  test('uploaded binary downloads back with matching bytes', async ({ page }) => {
    await setup(page)

    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
    await expect(page.getByTestId('file-row-test-image.png')).toBeVisible()

    await page.getByTestId('file-row-test-image.png').locator('[name="actions-dropdown"]').click()
    const [download] = await Promise.all([
      page.waitForEvent('download', { timeout: 30_000 }),
      page.locator('[name="download"]').first().click()
    ])
    expect(download.suggestedFilename()).toBe('test-image.png')

    // Bit-for-bit compare: read fixture bytes + downloaded bytes.
    const downloaded = await download.path()
    expect(downloaded).not.toBeNull()
    const downloadedBytes = fs.readFileSync(downloaded as string)
    const originalBytes = fs.readFileSync(imageFixture)

    expect(downloadedBytes.length).toBe(originalBytes.length)
    // Hash-compare instead of Buffer.equals for a cleaner assertion error.
    expect(downloadedBytes.equals(originalBytes)).toBe(true)
  })
})

test.describe('Versioned (editable) download path', () => {
  test('editable note downloads back as the current active version', async ({ page }) => {
    await setup(page)
    await createNoteFromBrowser(page, 'versioned-download.md')

    await openRawMarkdown(page)
    await typeRawMarkdown(page, '# versioned-download\n\nEnd-to-end test content.\n')
    await saveViaButton(page)

    // Trigger the same download flow the preview exposes — the toolbar's
    // action menu has a "Download" option.
    await page.locator('[name="md-actions"]').click()
    const [download] = await Promise.all([
      page.waitForEvent('download', { timeout: 30_000 }),
      page.getByRole('button', { name: 'Download', exact: true }).click()
    ])

    const downloadedPath = await download.path()
    expect(downloadedPath).not.toBeNull()
    const downloadedText = fs.readFileSync(downloadedPath as string, 'utf8')
    expect(downloadedText).toContain('End-to-end test content')
    expect(downloadedText).toContain('# versioned-download')
  })
})
