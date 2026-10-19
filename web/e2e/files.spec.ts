import { test, expect } from '@playwright/test'
import { BlobReader, Uint8ArrayWriter, ZipReader } from '@zip.js/zip.js'
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'fs/promises'
import { tmpdir } from 'os'
import { randomEmail, randomPassword, createUser, logout, loginAsUser } from './helpers/auth'
import path from 'path'

const imageFixture = path.join(__dirname, 'fixtures', 'test-image.png')
const batchFixtures = ['test-image.png', 'test-image2.png', 'test.pdf', 'test-video.mp4'].map(
  (name) => path.join(__dirname, 'fixtures', name)
)

async function setup(page: Parameters<typeof createUser>[0]) {
  const email = randomEmail()
  const password = randomPassword()
  // createUser registers and leaves the user fully logged in at '/'
  await createUser(page, email, password)
  return { email, password }
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

  test('creating a duplicate directory shows an error and keeps the dialog open', async ({ page }) => {
    await setup(page)

    await page.locator('[name="create-dir"]').click()
    await page.locator('#name').fill('Dup_Dir')
    await page.getByRole('button', { name: 'Create', exact: true }).click()
    await expect(page.getByTestId('file-row-Dup_Dir')).toBeVisible()

    await page.locator('[name="create-dir"]').click()
    await page.locator('#name').fill('Dup_Dir')
    await page.getByRole('button', { name: 'Create', exact: true }).click()

    const dialog = page.getByRole('dialog', { name: 'Create a folder' })
    await expect(dialog).toBeVisible()
    await expect(dialog.getByRole('alert')).toContainText('already exists')
  })

  test('dialogs paint above the page content in light theme', async ({ page }) => {
    await setup(page)
    await page.getByTestId('theme-toggle').click()

    await page.locator('[name="create-dir"]').click()
    const dialog = page.getByRole('dialog', { name: 'Create a folder' })
    await expect(dialog).toBeVisible()

    // The dialog card must win the paint order — a statically positioned
    // card once rendered underneath the page's own content in light theme.
    const onTop = await dialog.evaluate((el) => {
      const rect = el.getBoundingClientRect()
      const hit = document.elementFromPoint(rect.left + rect.width / 2, rect.top + 10)
      return hit !== null && el.contains(hit)
    })
    expect(onTop).toBe(true)
  })
})

test.describe('Upload', () => {
  test('can upload an image file and see the thumbnail', async ({ page }) => {
    const { email, password } = await setup(page)

    await page.setInputFiles('[name="upload-file-input"]', imageFixture)

    // Wait for the active upload to finish
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    // File row with thumbnail appears
    await expect(page.getByTestId('file-row-test-image.png')).toBeVisible()
    await expect(page.locator('img[name="thumbnail"][alt="test-image.png"]')).toBeVisible()

    // A fresh session starts with an empty store and a listing that
    // carries only `has_thumbnail` — the row fetches the blob from the
    // thumbnail route and decrypts it lazily.
    await logout(page)
    await loginAsUser(page, email, password)
    await expect(page.getByTestId('file-row-test-image.png')).toBeVisible()
    await expect(page.locator('img[name="thumbnail"][alt="test-image.png"]')).toBeVisible()
  })

  test('a file dropped outside the table still uploads', async ({ page }) => {
    await setup(page)

    const bytes = await readFile(imageFixture)
    const dataTransfer = await page.evaluateHandle((data) => {
      const dt = new DataTransfer()
      dt.items.add(new File([new Uint8Array(data)], 'test-image.png', { type: 'image/png' }))
      return dt
    }, [...bytes])

    const breadcrumb = page.getByLabel('Breadcrumb')
    await breadcrumb.dispatchEvent('dragover', { dataTransfer })
    await breadcrumb.dispatchEvent('drop', { dataTransfer })

    await expect(page.getByTestId('file-row-test-image.png')).toBeVisible({ timeout: 30_000 })
  })

  test('a file dropped while a modal is open does not upload', async ({ page }) => {
    await setup(page)

    const bytes = await readFile(imageFixture)
    const dataTransfer = await page.evaluateHandle((data) => {
      const dt = new DataTransfer()
      dt.items.add(new File([new Uint8Array(data)], 'test-image.png', { type: 'image/png' }))
      return dt
    }, [...bytes])

    await page.locator('[name="create-dir"]').click()
    const dialog = page.getByRole('dialog')
    await expect(dialog).toBeVisible()
    await dialog.dispatchEvent('dragover', { dataTransfer })
    await dialog.dispatchEvent('drop', { dataTransfer })
    await page.keyboard.press('Escape')
    await expect(dialog).toBeHidden()

    await expect(page.getByTestId('upload-active')).toBeHidden()
    await expect(page.getByTestId('file-row-test-image.png')).toHaveCount(0)
  })

  test('a batch of files stays visible in the queue and every file lands complete', async ({
    page
  }) => {
    await setup(page)

    const created: number[] = []
    page.on('response', (response) => {
      const url = new URL(response.url())
      if (response.request().method() === 'POST' && url.pathname === '/api/storage') {
        created.push(response.status())
      }
    })

    // Watched from before the files go in: small files upload side by side,
    // so the sentinel can be gone again by the time the last create is back.
    // Pinning it up first keeps a sentinel that never rendered from
    // satisfying the wait for it to go away.
    const sentinelShown = page.getByTestId('upload-active').waitFor({ timeout: 30_000 })

    await page.setInputFiles('[name="upload-file-input"]', batchFixtures)

    await sentinelShown
    await expect.poll(() => created.length, { timeout: 60_000 }).toBe(batchFixtures.length)

    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 120_000 })

    const listing = await page.request.get('/api/storage')
    expect(listing.ok()).toBeTruthy()

    const children = (await listing.json()).children as {
      chunks: number
      chunks_stored: number | null
      finished_upload_at: number | null
    }[]

    expect(children).toHaveLength(batchFixtures.length)
    for (const row of children) {
      expect(row.chunks_stored).toBe(row.chunks)
      expect(row.finished_upload_at).not.toBeNull()
    }
  })
})

test.describe('Many small files', () => {
  type Row = {
    name_hash: string
    mime: string
    chunks: number
    chunks_stored: number | null
    finished_upload_at: number | null
  }

  async function listing(page: Parameters<typeof createUser>[0], dirId?: string) {
    const response = await page.request.get(dirId ? `/api/storage?dir_id=${dirId}` : '/api/storage')
    expect(response.ok()).toBeTruthy()
    return (await response.json()).children as (Row & { id: string })[]
  }

  const isFinished = (row: Row) =>
    row.finished_upload_at !== null && row.chunks_stored === row.chunks

  test('a batch of small files is prepared and uploaded side by side', async ({ page }) => {
    await setup(page)

    const count = 50
    const files = Array.from({ length: count }, (_, i) => ({
      name: `small-${String(i).padStart(3, '0')}.txt`,
      mimeType: 'text/plain',
      buffer: Buffer.from(`file number ${i}\n`)
    }))

    let creating = 0
    let peakCreating = 0
    page.on('request', (request) => {
      if (request.method() === 'POST' && new URL(request.url()).pathname === '/api/storage') {
        creating++
        peakCreating = Math.max(peakCreating, creating)
      }
    })
    const settled = (request: { method(): string; url(): string }) => {
      if (request.method() === 'POST' && new URL(request.url()).pathname === '/api/storage') {
        creating--
      }
    }
    page.on('requestfinished', settled)
    page.on('requestfailed', settled)

    const started = Date.now()

    await page.setInputFiles('[name="upload-file-input"]', files)

    // The sentinel can drop between files while the next ones are still being
    // prepared, so the server's view is what says the batch is done.
    await expect
      .poll(async () => (await listing(page)).filter(isFinished).length, { timeout: 120_000 })
      .toBe(count)
    const elapsed = Date.now() - started

    expect(await listing(page)).toHaveLength(count)
    await expect(page.getByTestId('upload-active')).toHaveCount(0)

    expect(peakCreating).toBeGreaterThan(1)

    // One file at a time, each waiting for the queue's next one-second tick,
    // put a floor of `count` seconds under this; side by side it is a few.
    expect(elapsed).toBeLessThan(count * 1000 * 0.6)
  })

  test('a folder of small files keeps its structure', async ({ page }) => {
    await setup(page)

    const root = await mkdtemp(path.join(tmpdir(), 'hoodik-e2e-'))
    const top = path.join(root, 'tree')
    const layout: Record<string, number> = { '': 6, a: 6, 'a/deep': 6, b: 6 }
    for (const [dir, files] of Object.entries(layout)) {
      await mkdir(path.join(top, dir), { recursive: true })
      for (let i = 0; i < files; i++) {
        await writeFile(path.join(top, dir, `f${i}.txt`), `${dir}/${i}\n`)
      }
    }

    // Names are encrypted, so the shape is what can be checked from here:
    // tree/ holds 6 files and 2 folders, a/ holds 6 files and 1 folder.
    const shape = async (dirId?: string) => {
      const children = await listing(page, dirId)
      for (const row of children.filter((c) => c.mime !== 'dir')) {
        expect(isFinished(row)).toBe(true)
      }
      return {
        files: children.filter((c) => c.mime !== 'dir').length,
        dirs: children.filter((c) => c.mime === 'dir')
      }
    }

    try {
      await page.setInputFiles('[name="upload-folder-input"]', top)

      // Polled rather than gated on the sentinel, which can drop while the
      // next folder is being created and nothing is queued yet.
      await expect(async () => {
        const rootLevel = await shape()
        expect(rootLevel).toMatchObject({ files: 0 })
        expect(rootLevel.dirs).toHaveLength(1)

        const tree = await shape(rootLevel.dirs[0].id)
        expect(tree.files).toBe(6)
        expect(tree.dirs).toHaveLength(2)

        const levels = await Promise.all(tree.dirs.map((d) => shape(d.id)))
        const withSub = levels.find((l) => l.dirs.length === 1)
        const leaf = levels.find((l) => l.dirs.length === 0)
        expect(withSub?.files).toBe(6)
        expect(leaf?.files).toBe(6)

        const deep = await shape(withSub!.dirs[0].id)
        expect(deep).toMatchObject({ files: 6, dirs: [] })
      }).toPass({ timeout: 120_000 })

      await expect(page.getByTestId('upload-active')).toHaveCount(0)
    } finally {
      await rm(root, { recursive: true, force: true })
    }
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

  test('can download a folder as a zip archive', async ({ page }) => {
    // Forces the in-memory fallback, which arrives as an ordinary download.
    await page.addInitScript(() => {
      delete (window as { showSaveFilePicker?: unknown }).showSaveFilePicker
    })
    await setup(page)
    await createFolderWithImage(page, 'Photos')

    await page.getByTestId('file-row-Photos').locator('input[type="checkbox"]').check()
    await expect(page.getByTitle('Add to download queue')).toBeVisible()
    await page.getByTestId('file-row-Photos').locator('input[type="checkbox"]').uncheck()

    await page.getByTestId('file-row-Photos').locator('[name="actions-dropdown"]').click()

    const [download] = await Promise.all([
      page.waitForEvent('download', { timeout: 60_000 }),
      page.locator('[name="download"]').first().click(),
    ])

    expect(download.suggestedFilename()).toBe('Photos.zip')
    await expectArchiveOfImage(await readFile(await download.path()), 'Photos')
  })

  test('downloads several selected folders as one zip archive', async ({ page }) => {
    await page.addInitScript(() => {
      delete (window as { showSaveFilePicker?: unknown }).showSaveFilePicker
    })
    await setup(page)
    await createFolderWithImage(page, 'Photos')
    await createFolderWithImage(page, 'Scans')

    await page.getByTestId('file-row-Photos').locator('input[type="checkbox"]').check()
    await page.getByTestId('file-row-Scans').locator('input[type="checkbox"]').check()

    const [download] = await Promise.all([
      page.waitForEvent('download', { timeout: 60_000 }),
      page.getByTitle('Add to download queue').click(),
    ])

    expect(download.suggestedFilename()).toMatch(/^download-\d{8}T\d{6}\.zip$/)

    const reader = new ZipReader(new BlobReader(new Blob([await readFile(await download.path())])))
    const names = (await reader.getEntries()).map((entry) => entry.filename).sort()
    expect(names).toEqual(['Photos/test-image.png', 'Scans/test-image.png'])
    await reader.close()
  })

  test('streams a folder archive to the file picked in the save dialog', async ({ page }) => {
    // The native dialog can't be driven from a test; an origin-private file
    // is still a real handle with a real writable stream.
    await page.addInitScript(() => {
      ;(window as unknown as { showSaveFilePicker: unknown }).showSaveFilePicker = async (
        options: { suggestedName: string }
      ) => {
        ;(window as unknown as { pickedName: string }).pickedName = options.suggestedName
        const root = await navigator.storage.getDirectory()
        return root.getFileHandle('picked.zip', { create: true })
      }
    })
    await setup(page)
    await createFolderWithImage(page, 'Photos')

    await page.getByTestId('file-row-Photos').locator('[name="actions-dropdown"]').click()
    await page.locator('[name="download"]').first().click()

    // The writable only commits once the archive is closed.
    const readPicked = () =>
      page.evaluate(async () => {
        const root = await navigator.storage.getDirectory()
        // Absent until the stubbed picker has run.
        const handle = await root.getFileHandle('picked.zip').catch(() => undefined)
        if (!handle) return []
        return Array.from(new Uint8Array(await (await handle.getFile()).arrayBuffer()))
      })
    await expect.poll(async () => (await readPicked()).length, { timeout: 60_000 }).toBeGreaterThan(0)

    expect(await page.evaluate(() => (window as unknown as { pickedName: string }).pickedName)).toBe(
      'Photos.zip'
    )
    await expectArchiveOfImage(Buffer.from(await readPicked()), 'Photos')
  })
})

async function createFolderWithImage(page: Parameters<typeof createUser>[0], name: string) {
  await page.locator('[name="create-dir"]').click()
  await page.locator('#name').fill(name)
  await page.getByRole('button', { name: 'Create', exact: true }).click()
  await page.getByTestId(`file-row-${name}`).dblclick()
  await expect(page).toHaveURL(/[0-9a-f-]{36}/)

  await page.setInputFiles('[name="upload-file-input"]', imageFixture)
  await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
  await expect(page.getByTestId('file-row-test-image.png')).toBeVisible()

  await page.getByLabel('Breadcrumb').getByRole('link', { name: 'My Files' }).click()
  await expect(page.getByTestId(`file-row-${name}`)).toBeVisible()
}

async function expectArchiveOfImage(archive: Buffer, folder: string) {
  const reader = new ZipReader(new BlobReader(new Blob([archive])))
  const entries = await reader.getEntries()
  expect(entries.map((entry) => entry.filename)).toEqual([`${folder}/test-image.png`])

  const content = await entries[0].getData!(new Uint8ArrayWriter())
  expect(Buffer.from(content).equals(await readFile(imageFixture))).toBe(true)
  await reader.close()
}

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

test.describe('Selection counter', () => {
  test('shows the count of selected items in the toolbar and updates as selection changes', async ({ page }) => {
    await setup(page)

    for (const name of ['Folder_A', 'Folder_B']) {
      await page.locator('[name="create-dir"]').click()
      await page.locator('#name').fill(name)
      await page.getByRole('button', { name: 'Create', exact: true }).click()
      await expect(page.getByTestId(`file-row-${name}`)).toBeVisible()
    }

    const counter = page.getByTestId('files-selected-count')
    await expect(counter).toHaveCount(0)

    await page.getByTestId('file-row-Folder_A').locator('input[type="checkbox"]').check()
    await expect(counter).toHaveText(/^1 selected$/)

    await page.getByTestId('file-row-Folder_B').locator('input[type="checkbox"]').check()
    await expect(counter).toHaveText(/^2 selected$/)

    await page.getByTestId('file-row-Folder_A').locator('input[type="checkbox"]').uncheck()
    await expect(counter).toHaveText(/^1 selected$/)

    await page.getByTestId('file-row-Folder_B').locator('input[type="checkbox"]').uncheck()
    await expect(counter).toHaveCount(0)
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

  // Ticking a checkbox leaves focus on it, which must not swallow the shortcut
  for (const key of ['Delete', 'ControlOrMeta+Backspace']) {
    test(`${key} deletes the rows just ticked with their checkboxes`, async ({ page }) => {
      await setup(page)

      for (const name of ['Folder_A', 'Folder_B']) {
        await page.locator('[name="create-dir"]').click()
        await page.locator('#name').fill(name)
        await page.getByRole('button', { name: 'Create', exact: true }).click()
        await expect(page.getByTestId(`file-row-${name}`)).toBeVisible()
      }

      await page.getByTestId('file-row-Folder_A').locator('input[type="checkbox"]').check()
      await page.getByTestId('file-row-Folder_B').locator('input[type="checkbox"]').check()
      await page.keyboard.press(key)

      await page.getByTestId('delete-confirm-count').fill('2')
      await page.getByRole('button', { name: 'Yes, delete' }).click()

      await expect(page.getByTestId('file-row-Folder_A')).not.toBeVisible()
      await expect(page.getByTestId('file-row-Folder_B')).not.toBeVisible()
    })
  }
})
