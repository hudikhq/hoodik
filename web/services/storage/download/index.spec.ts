import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { store as useDownload } from './index'
import { downloadAsZip, pickSaveTarget, type FolderDownloadHooks } from './folder'
import type { AppFile, DownloadAppFile } from 'types'

vi.mock('!/logger', () => ({
  debug: vi.fn(),
  error: vi.fn(),
  info: vi.fn(),
  warn: vi.fn()
}))

vi.mock('../..', () => ({
  errorIntoWorkerError: vi.fn((e) => e),
  localDateFromUtcString: vi.fn(() => new Date()),
  utcStringFromLocal: vi.fn(() => new Date().toISOString()),
  uuidv4: vi.fn(() => `uuid-${Math.random()}`)
}))

vi.mock('../../constants', () => ({
  FILES_DOWNLOADING_AT_ONE_TIME: 3,
  KEEP_FINISHED_DOWNLOADS_FOR_MINUTES: 5
}))

vi.mock('../workers', () => ({
  startFileDownload: vi.fn()
}))

vi.mock('./sync', () => ({
  downloadAndDecryptStream: vi.fn(),
  downloadAndDecrypt: vi.fn()
}))

vi.mock('./folder', () => ({
  downloadAsZip: vi.fn(),
  pickSaveTarget: vi.fn(async () => undefined),
  saveBlob: vi.fn()
}))

vi.mock('../../crypto', () => ({
  store: () => ({ keypair: { input: 'private' } })
}))

vi.mock('..', () => ({
  meta: { get: vi.fn() }
}))

function makeFile(id: string, name: string, size = 10_000): DownloadAppFile {
  return {
    id,
    name,
    size,
    temporaryId: `tmp-${id}`,
    // file_id is the parent directory id — use a value that won't match the mock storage dir
    file_id: `parent-of-${id}`,
    key: new Uint8Array(32),
    downloadedBytes: 0
  } as unknown as DownloadAppFile
}

const mockStorage = {
  dir: { id: 'current-dir' },
  upsertItem: vi.fn()
} as any

describe('download store — progress()', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('splice(-1,1) regression: does not remove an existing running file on first progress event for a new file', async () => {
    const dl = useDownload()

    const fileA = makeFile('file-a', 'already-running.txt')
    const fileB = makeFile('file-b', 'new-download.txt')

    // fileA is already mid-download
    dl.running.push(fileA)
    expect(dl.running).toHaveLength(1)

    // fileB receives its very first progress event (index === -1, not yet in running).
    // Bug: splice(-1, 1) removes the last element — fileA — instead of doing nothing.
    await dl.progress(mockStorage, fileB, 500)

    // fileA must still be in running
    expect(dl.running.find((f: DownloadAppFile) => f.id === 'file-a')).toBeDefined()
    // fileB should now be in running (500 < 10_000, not done yet)
    expect(dl.running.find((f: DownloadAppFile) => f.id === 'file-b')).toBeDefined()
  })

  it('removes the correct file when a subsequent progress event arrives for a file already in running', async () => {
    const dl = useDownload()

    const fileA = makeFile('file-a', 'a.txt')
    const fileB = makeFile('file-b', 'b.txt')

    dl.running.push(fileA)
    dl.running.push(fileB)
    expect(dl.running).toHaveLength(2)

    // fileB's second progress event — it IS in running, so splice(correctIndex, 1) is used
    await dl.progress(mockStorage, fileB, 500)

    // fileA must still be present
    expect(dl.running.find((f: DownloadAppFile) => f.id === 'file-a')).toBeDefined()
    // fileB is re-added at front (unshift) because 500 < 10_000
    expect(dl.running.find((f: DownloadAppFile) => f.id === 'file-b')).toBeDefined()
    expect(dl.running).toHaveLength(2)
  })

  it('moves a file to done when downloadedBytes reaches its size', async () => {
    const dl = useDownload()

    const file = makeFile('file-c', 'c.txt', 1_000)
    dl.running.push(file)

    await dl.progress(mockStorage, file, 1_000)

    expect(dl.running.find((f: DownloadAppFile) => f.id === 'file-c')).toBeUndefined()
    expect(dl.done.find((f: DownloadAppFile) => f.id === 'file-c')).toBeDefined()
  })

  it('keeps a staged file in running when its bytes reach the total', async () => {
    const download = useDownload()
    const file = makeFile('f1', 'staged.bin')

    await download.progress(mockStorage, file, file.size as number, undefined, 'processing')

    // The pipeline finished receiving, but the blob hasn't reached the
    // browser — the row stays visible as running with its stage set.
    expect(download.done).toHaveLength(0)
    expect(download.running).toHaveLength(1)
    expect((download.running[0] as DownloadAppFile).stage).toBe('processing')
  })

  it('finish() moves a staged file to done exactly once', async () => {
    const download = useDownload()
    const file = makeFile('f1', 'staged.bin')

    await download.progress(mockStorage, file, file.size as number, undefined, 'processing')
    download.finish(file)
    download.finish(file)

    expect(download.running).toHaveLength(0)
    expect(download.done).toHaveLength(1)
    expect((download.done[0] as DownloadAppFile).stage).toBeUndefined()
    expect((download.done[0] as DownloadAppFile).finished_downloading_at).toBeTruthy()
  })

  it('moves a file to failed on error and does not touch other running files', async () => {
    const dl = useDownload()

    const fileA = makeFile('file-a', 'a.txt')
    const fileB = makeFile('file-b', 'b.txt')

    dl.running.push(fileA)
    dl.running.push(fileB)

    await dl.progress(mockStorage, fileB, 0, new Error('network error'))

    expect(dl.running.find((f: DownloadAppFile) => f.id === 'file-a')).toBeDefined()
    expect(dl.running.find((f: DownloadAppFile) => f.id === 'file-b')).toBeUndefined()
    expect(dl.failed.find((f: DownloadAppFile) => f.id === 'file-b')).toBeDefined()
  })
})

describe('download store — folders', () => {
  const folder = { id: 'folder-1', name: 'Photos', mime: 'dir' } as AppFile
  let runs: FolderDownloadHooks[]
  let interval: ReturnType<typeof setInterval>

  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    // Only the queue's interval: `downloadFolder` imports the crypto store
    // lazily, which needs real ticks to resolve.
    vi.useFakeTimers({ toFake: ['setInterval'] })
    runs = []
    // Each run reports once, then stays in flight like a stalled chunk fetch.
    vi.mocked(downloadAsZip).mockImplementation(async (_keypair, _roots, hooks = {}) => {
      runs.push(hooks)
      hooks.onProgress?.('downloading', 1, 10)
      return new Promise(() => {})
    })
  })

  afterEach(() => {
    clearInterval(interval)
    vi.useRealTimers()
  })

  async function tick(expectedRuns: number) {
    await vi.advanceTimersByTimeAsync(1000)
    for (let i = 0; runs.length < expectedRuns && i < 100; i++) {
      await new Promise((resolve) => setTimeout(resolve, 1))
    }
    expect(runs).toHaveLength(expectedRuns)
  }

  it('names the queue row after the archive', async () => {
    const dl = useDownload()

    await dl.push(folder)

    expect(dl.waiting[0].name).toBe('Photos.zip')
    expect(dl.waiting[0].items?.map((item) => item.name)).toEqual(['Photos'])
  })

  it('names the queue row after the file picked in the save dialog', async () => {
    vi.mocked(pickSaveTarget).mockResolvedValueOnce({ name: 'holiday.zip' } as FileSystemFileHandle)
    const dl = useDownload()

    await dl.push(folder)

    expect(dl.waiting[0].name).toBe('holiday.zip')
  })

  it('keeps a cancelled folder run cancelled when the folder is downloaded again', async () => {
    const dl = useDownload()
    interval = await dl.start(mockStorage, {} as any)

    await dl.push(folder)
    await tick(1)
    const [first] = dl.running
    await dl.cancel(mockStorage, first)

    await dl.push(folder)
    await tick(2)

    expect(runs[0].isCancelled?.()).toBe(true)
    expect(runs[1].isCancelled?.()).toBe(false)

    // A late report from the stopped run must not take over the new row.
    runs[0].onProgress?.('downloading', 9, 10)
    expect(dl.running).toHaveLength(1)
    expect(dl.running[0].temporaryId).not.toBe(first.temporaryId)
  })

  it('retries a cancelled folder as a new run', async () => {
    const dl = useDownload()
    interval = await dl.start(mockStorage, {} as any)

    await dl.push(folder)
    await tick(1)
    await dl.cancel(mockStorage, dl.running[0])

    dl.retry(dl.failed[0])
    await tick(2)

    expect(runs[0].isCancelled?.()).toBe(true)
    expect(runs[1].isCancelled?.()).toBe(false)
  })
})
