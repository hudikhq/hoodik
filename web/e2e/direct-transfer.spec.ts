import { test, expect } from '@playwright/test'
import { randomEmail, randomPassword, createUser } from './helpers/auth'
import {
  createNoteFromBrowser,
  openRawMarkdown,
  typeRawMarkdown,
  saveViaButton,
  openHistory,
  previewVersion
} from './helpers/notes'
import path from 'path'

const imageFixture = path.join(__dirname, 'fixtures', 'test-image.png')

/**
 * Chunks reaching the browser from the storage bucket instead of from Hoodik.
 *
 * Only meaningful under `just e2e-direct`, which points the server at MinIO
 * and switches the feature on. The default `just e2e` run leaves the
 * capability off, and these skip rather than fail: the relaying path is not a
 * degraded mode, it is the supported default for every deployment whose
 * bucket cannot serve browsers.
 */
test.describe('Direct transfer', () => {
  test.beforeEach(async ({ page }) => {
    const capabilities = await page.request.get('/api/capabilities')
    const body = await capabilities.json()
    test.skip(
      body.direct_transfer !== true,
      'server does not advertise direct transfer — run under `just e2e-direct`'
    )
  })

  test('the server advertises the capability and reports no blockers', async ({ page }) => {
    const readiness = await page.request.get('/api/readiness')
    const body = await readiness.json()

    expect(body.status).toBe('ready')
    expect(body.direct_transfer).toBe(true)
    expect(body.direct_transfer_blockers).toEqual([])
  })

  test('a preview fetches chunks from the bucket, not from Hoodik', async ({ page }) => {
    const email = randomEmail()
    const password = randomPassword()
    await createUser(page, email, password)

    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 60_000 })
    await expect(page.getByTestId('file-row-test-image.png')).toBeVisible()

    // Everything the page asks for from here on, so the assertions can talk
    // about where bytes came from rather than about timing.
    const requested: string[] = []
    page.on('request', (request) => requested.push(request.url()))

    await page.getByTestId('file-row-test-image.png').dblclick()
    await expect(page.locator('img[name="original"]')).toBeVisible({ timeout: 60_000 })

    const manifest = requested.filter((url) => url.includes('/chunk-urls'))
    expect(manifest.length, 'the client should have asked for a manifest').toBeGreaterThan(0)

    const fromBucket = requested.filter((url) => url.includes(':9000/'))
    expect(fromBucket.length, 'chunks should have been fetched from the bucket').toBeGreaterThan(0)

    // The relaying route must not have been used for chunk bytes. A request
    // for the file's metadata is fine; one carrying `?chunk=` is not.
    const relayed = requested.filter((url) => /\/api\/storage\/[0-9a-f-]{36}\?.*chunk=/.test(url))
    expect(relayed, 'no chunk should have been relayed through the server').toEqual([])
  })

  // Each tail gets its own test on purpose. The preview above passed for a
  // long stretch during which downloading a file and uploading one both
  // relayed every byte, because a preview decrypts on the main thread and
  // never goes near the workers these two exercise.
  test('an upload writes chunks into the bucket, not through Hoodik', async ({ page }) => {
    const email = randomEmail()
    const password = randomPassword()
    await createUser(page, email, password)

    const requested: { method: string; url: string }[] = []
    page.on('request', (request) => requested.push({ method: request.method(), url: request.url() }))

    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 60_000 })
    await expect(page.getByTestId('file-row-test-image.png')).toBeVisible()

    expect(
      requested.filter((r) => r.url.includes('/upload-urls')).length,
      'the client should have asked for upload URLs'
    ).toBeGreaterThan(0)

    // Asserted positively, so a transport this harness cannot observe fails
    // the test instead of passing it by silence.
    expect(
      requested.filter((r) => r.method === 'PUT' && r.url.includes(':9000/')).length,
      'chunks should have been written to the bucket'
    ).toBeGreaterThan(0)

    expect(
      requested.filter((r) => r.url.includes('/finalize')).length,
      'a direct upload has to be committed explicitly'
    ).toBeGreaterThan(0)

    // A relayed upload is a POST to the file's own route carrying `?chunk=`.
    const relayed = requested.filter(
      (r) => r.method === 'POST' && /\/api\/storage\/[0-9a-f-]{36}\?.*chunk=/.test(r.url)
    )
    expect(relayed, 'no chunk should have been relayed through the server').toEqual([])
  })

  test('downloading a file to disk fetches its chunks from the bucket', async ({ page }) => {
    const email = randomEmail()
    const password = randomPassword()
    await createUser(page, email, password)

    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 60_000 })
    await expect(page.getByTestId('file-row-test-image.png')).toBeVisible()

    const requested: string[] = []
    page.on('request', (request) => requested.push(request.url()))

    await page.getByTestId('file-row-test-image.png').locator('[name="actions-dropdown"]').click()
    const [download] = await Promise.all([
      page.waitForEvent('download'),
      page.locator('[name="download"]').first().click()
    ])
    expect(download.suggestedFilename()).toBe('test-image.png')

    expect(
      requested.filter((url) => url.includes('/chunk-urls')).length,
      'the client should have asked for a manifest'
    ).toBeGreaterThan(0)

    expect(
      requested.filter((url) => url.includes(':9000/')).length,
      'chunks should have been fetched from the bucket'
    ).toBeGreaterThan(0)

    const relayed = requested.filter((url) => /\/api\/storage\/[0-9a-f-]{36}\?.*chunk=/.test(url))
    expect(relayed, 'no chunk should have been relayed through the server').toEqual([])
  })

  test('bucket requests carry no session credentials', async ({ page }) => {
    const email = randomEmail()
    const password = randomPassword()
    await createUser(page, email, password)

    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 60_000 })

    const leaked: string[] = []
    page.on('request', (request) => {
      if (!request.url().includes(':9000/')) return
      const headers = request.headers()
      for (const name of ['cookie', 'authorization', 'x-auth-refresh']) {
        if (headers[name]) leaked.push(`${name} on ${request.url()}`)
      }
    })

    await page.getByTestId('file-row-test-image.png').dblclick()
    await expect(page.locator('img[name="original"]')).toBeVisible({ timeout: 60_000 })

    // The whole point of the ChunkTarget split, asserted against the wire
    // rather than against the type that is supposed to guarantee it.
    expect(leaked, 'a request to the bucket carried session credentials').toEqual([])
  })

  test('a finished file cannot be signed for overwriting', async ({ page }) => {
    const email = randomEmail()
    const password = randomPassword()
    await createUser(page, email, password)

    const uploadUrlRequests: string[] = []
    page.on('request', (request) => {
      if (request.url().includes('/upload-urls')) uploadUrlRequests.push(request.url())
    })

    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 60_000 })
    await expect(page.getByTestId('file-row-test-image.png')).toBeVisible()

    const manifestUrl = uploadUrlRequests[0]
    expect(manifestUrl, 'the upload should have asked for upload URLs').toBeTruthy()

    // Chunks are write-once: with every chunk stored and no edit in flight,
    // the server must refuse to sign a PUT at the live ciphertext — S3
    // overwrites unconditionally, so a signed URL here IS the overwrite.
    const again = await page.request.post(new URL(manifestUrl).pathname, {
      data: { chunks: [{ chunk: 0, size: 1024 }] }
    })
    expect(again.status()).toBe(422)
    expect(await again.text()).toContain('chunk_already_exists')
  })

  test('a public link serves its bytes from the bucket', async ({ page, browser }) => {
    const email = randomEmail()
    const password = randomPassword()
    await createUser(page, email, password)

    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 60_000 })
    await expect(page.getByTestId('file-row-test-image.png')).toBeVisible()

    await page.getByTestId('file-row-test-image.png').locator('[name="actions-dropdown"]').click()
    await page.locator('[name="sharing"]').first().click()
    await page.getByTestId('sharing-modal-tab-link').click()
    await page.getByTestId('sharing-link-create').click()
    const linkUrl = await page.locator('input[name="link"]').inputValue()
    expect(linkUrl).toBeTruthy()

    const anonContext = await browser.newContext()
    const anonPage = await anonContext.newPage()

    const requested: string[] = []
    anonPage.on('request', (request) => requested.push(request.url()))

    await anonPage.goto(linkUrl)
    await expect(anonPage.locator('img[name="original"]')).toBeVisible({ timeout: 60_000 })

    expect(
      requested.filter((url) => url.includes('/chunk-urls')).length,
      'the link page should have asked for a manifest'
    ).toBeGreaterThan(0)
    expect(
      requested.filter((url) => url.includes(':9000/')).length,
      'link chunks should have been fetched from the bucket'
    ).toBeGreaterThan(0)

    const relayed = requested.filter((url) => /\/api\/links\/[0-9a-f-]{36}\?.*chunk=/.test(url))
    expect(relayed, 'no link chunk should have been relayed through the server').toEqual([])

    await anonContext.close()
  })

  test('previewing a historical version fetches its chunks from the bucket', async ({ page }) => {
    const email = randomEmail()
    const password = randomPassword()
    await createUser(page, email, password)

    await createNoteFromBrowser(page, 'direct-versions.md')
    await openRawMarkdown(page)
    await typeRawMarkdown(page, 'A\n')
    await saveViaButton(page)
    await typeRawMarkdown(page, 'B\n')
    await saveViaButton(page)

    const requested: string[] = []
    page.on('request', (request) => requested.push(request.url()))

    await openHistory(page)
    await previewVersion(page, 2)

    expect(
      requested.filter((url) => /\/versions\/\d+\/chunk-urls/.test(url)).length,
      'the preview should have asked for a version manifest'
    ).toBeGreaterThan(0)
    expect(
      requested.filter((url) => url.includes(':9000/')).length,
      'version chunks should have been fetched from the bucket'
    ).toBeGreaterThan(0)

    const relayed = requested.filter((url) => /\/versions\/\d+\?.*chunk=/.test(url))
    expect(relayed, 'no version chunk should have been relayed through the server').toEqual([])
  })
})
