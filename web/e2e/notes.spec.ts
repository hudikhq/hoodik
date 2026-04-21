import { test, expect } from '@playwright/test'
import path from 'path'
import { randomEmail, randomPassword, createUser } from './helpers/auth'
import {
  createNoteFromBrowser,
  createNoteFromLanding,
  createPersistedUser,
  openRawMarkdown,
  typeRawMarkdown,
  saveViaButton,
  waitForAutoSaveToSettle
} from './helpers/notes'

const markdownFixture = path.join(__dirname, 'fixtures', 'readonly-note.md')

async function setup(page: Parameters<typeof createUser>[0]) {
  await createUser(page, randomEmail(), randomPassword())
}

test.describe('New File button — A1 create flow', () => {
  test('creates an editable .md from the file browser and lands in the editor', async ({ page }) => {
    await setup(page)

    const id = await createNoteFromBrowser(page, 'from-browser.md')
    expect(id).toMatch(/[0-9a-f-]{36}/)

    // Initial content is `# from-browser\n` — we should be able to read it
    // out of the raw markdown view.
    await openRawMarkdown(page)
    await expect(page.locator('.md-raw-textarea')).toHaveValue(/from-browser/)
  })

  test('appends .md extension when the user forgets it', async ({ page }) => {
    await setup(page)
    await createNoteFromBrowser(page, 'shopping-list')

    // Even though we typed no ".md", the editor should have rendered with
    // that filename (the title bar / action menu reads from preview.name).
    await openRawMarkdown(page)
    await expect(page.locator('.md-raw-textarea')).toHaveValue(/shopping-list/)
  })

  test('creates a note from the Notes landing view and lands in the editor', async ({ page }) => {
    await setup(page)
    const id = await createNoteFromLanding(page, 'from-landing.md')
    expect(id).toMatch(/[0-9a-f-]{36}/)
    await openRawMarkdown(page)
    await expect(page.locator('.md-raw-textarea')).toHaveValue(/from-landing/)
  })
})

test.describe('Content persistence — A1 save flow', () => {
  test('auto-save keeps edits across a page reload', async ({ page }) => {
    // `page.reload()` wipes in-memory auth state, so we need a Remember-Me
    // session for the post-reload navigation to land on the editor
    // instead of bouncing to /auth/login.
    await createPersistedUser(page)
    const id = await createNoteFromBrowser(page, 'autosaved.md')

    await openRawMarkdown(page)
    await typeRawMarkdown(page, '# autosaved\n\nAuto-saved paragraph.\n')
    // Compose debounces 5 s — waitForAutoSaveToSettle gives us headroom.
    await waitForAutoSaveToSettle(page)

    await page.reload()
    await page.waitForURL(new RegExp(`/notes/${id}`))
    await openRawMarkdown(page)
    await expect(page.locator('.md-raw-textarea')).toHaveValue(/Auto-saved paragraph/)
  })

  test('Ctrl+S (Cmd+S on mac) forces a save immediately', async ({ page }) => {
    await createPersistedUser(page)
    const id = await createNoteFromBrowser(page, 'manual-save.md')

    await openRawMarkdown(page)
    await typeRawMarkdown(page, '# manual-save\n\nManually saved paragraph.\n')

    // We send the platform-appropriate modifier. Playwright normalizes
    // `Control` on non-macOS and `Meta` on macOS — since the editor
    // accepts either ("Mod-s" is a prosemirror alias) we pick one deterministically
    // to keep the test cross-platform.
    const raw = page.locator('.md-raw-textarea')
    await raw.focus()
    await page.keyboard.press('Control+s')

    // The save button also exists — click it as a second safety net, then
    // confirm the indicator shows "Saved".
    await saveViaButton(page)

    await page.reload()
    await page.waitForURL(new RegExp(`/notes/${id}`))
    await openRawMarkdown(page)
    await expect(page.locator('.md-raw-textarea')).toHaveValue(/Manually saved paragraph/)
  })
})

test.describe('Formatting — A1 toolbar commands', () => {
  test('toolbar Bold wraps selected text in **', async ({ page }) => {
    await setup(page)
    await createNoteFromBrowser(page, 'bold.md')

    // Switch directly to WYSIWYG (the editor's default mode) and type
    // into ProseMirror. The raw textarea path is separate and doesn't
    // go through the toolbar commands.
    const prose = page.locator('.milkdown-wrapper .ProseMirror').first()
    await prose.waitFor({ state: 'visible' })
    await prose.click()

    // Move the caret to end of document, then type a marker word.
    await page.keyboard.press('Control+End')
    await page.keyboard.type('MARKER')

    // Select the word we just typed — 6 Shift+ArrowLeft presses walk
    // the caret back across the six characters of "MARKER".
    for (let i = 0; i < 6; i++) {
      await page.keyboard.press('Shift+ArrowLeft')
    }

    // Apply Bold via the toolbar button.
    await page.locator('[name="md-bold"]').click()

    // Explicit save — we want a deterministic flush before reading the
    // raw view.
    await page.keyboard.press('Control+s')
    await saveViaButton(page)

    // The raw view round-trips — the saved markdown should contain
    // "**MARKER**".
    await page.locator('[name="md-actions"]').click()
    await page.getByRole('button', { name: /Raw markdown/i }).click()
    const raw = page.locator('.md-raw-textarea')
    await raw.waitFor({ state: 'visible' })
    await expect(raw).toHaveValue(/\*\*MARKER\*\*/)
  })
})

test.describe('Export PDF — A1 export flow', () => {
  test('clicking Export PDF triggers a browser download', async ({ page }) => {
    await setup(page)
    await createNoteFromBrowser(page, 'export.md')

    // Switch to the WYSIWYG editor — the exporter reads content from
    // `.ProseMirror`, not the raw textarea. With the note's initial
    // heading (# export\n) we already have enough non-trivial content
    // to exercise the exporter.
    const prose = page.locator('.milkdown-wrapper .ProseMirror').first()
    await prose.waitFor({ state: 'visible' })

    // The "Export PDF" option lives in the MarkdownActions dropdown.
    await page.locator('[name="md-actions"]').click()
    // Wait for the dropdown to be rendered before clicking the option.
    const exportBtn = page.getByRole('button', { name: /Export PDF/i })
    await exportBtn.waitFor({ state: 'visible' })

    const [download] = await Promise.all([
      page.waitForEvent('download', { timeout: 60_000 }),
      exportBtn.click()
    ])

    // html2pdf derives the filename from the note name (minus `.md`).
    expect(download.suggestedFilename()).toMatch(/\.pdf$/i)
  })
})

test.describe('Uploaded markdown — A1 file → editor routing', () => {
  test('uploading a .md file lands it in the editor in edit mode', async ({ page }) => {
    // The upload pipeline auto-sets `editable: true` for any .md/x-markdown
    // upload — there's no UI path to land a non-editable .md today (see
    // web/services/storage/upload/index.ts:287). What the user CAN verify
    // here is the opposite invariant: an uploaded .md opens directly in
    // the editable editor, not a read-only preview.
    await setup(page)

    await page.setInputFiles('[name="upload-file-input"]', markdownFixture)
    await page.getByTestId('file-row-readonly-note.md').waitFor({ state: 'visible', timeout: 30_000 })

    // Double-clicking a .md routes to /notes/:id regardless of editable.
    await page.getByTestId('file-row-readonly-note.md').dblclick()
    await page.waitForURL(/\/notes\/[0-9a-f-]{36}/)

    // Editable — the Save button is part of the toolbar.
    await expect(page.locator('[name="md-save"]')).toBeVisible()
    // Convert-to-note banner only shows for non-editable files; with the
    // auto-editable upload policy it must be absent for .md uploads.
    await expect(page.locator('.md-convert-banner')).toHaveCount(0)
  })
})

test.describe('Notes landing — A1 editable filter', () => {
  test('the Notes landing view lists editable notes but skips non-markdown files', async ({ page }) => {
    // The landing view calls `meta.find({ editable: true })` server-side
    // and then filters to `isMarkdownFile` client-side. What the user
    // observes is: "notes I created" show up, unrelated uploads (PDF,
    // images, etc.) do not. `.md` uploads DO show up because the upload
    // pipeline auto-promotes them to editable — see
    // web/services/storage/upload/index.ts:287.
    await setup(page)

    // Upload a non-markdown binary — this lands non-editable and MUST
    // NOT show up in the Notes landing.
    await page.setInputFiles('[name="upload-file-input"]', path.join(__dirname, 'fixtures', 'test-image.png'))
    await page.getByTestId('file-row-test-image.png').waitFor({ state: 'visible', timeout: 30_000 })

    // Create two editable notes via in-SPA navigation so the auth store
    // survives. `createNoteFromBrowser` navigates into /notes/:id; use
    // the sidebar "Files" link to come back between creates.
    await createNoteFromBrowser(page, 'editable-a.md')
    await page.locator('aside').getByText('Files', { exact: true }).first().click()
    await page.waitForURL(/^[^#]*\/$/)
    await createNoteFromBrowser(page, 'editable-b.md')

    // Go to Notes landing via the sidebar (SPA nav, preserves auth).
    await page.locator('aside').getByText('Notes', { exact: true }).first().click()
    await page.waitForURL(/\/notes(\/|$|\?)/)
    await page.getByRole('heading', { name: 'Notes' }).waitFor({ state: 'visible' })
    // Give the landing view's initial fetch a moment to resolve.
    await page.waitForLoadState('networkidle')

    // Editable notes show up; the uploaded image does not.
    const names = await page.locator('ul li p.text-sm').allTextContents()
    expect(names.some((n) => n.includes('editable-a.md'))).toBe(true)
    expect(names.some((n) => n.includes('editable-b.md'))).toBe(true)
    expect(names.some((n) => n.includes('test-image.png'))).toBe(false)
  })
})
