import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { store as useDownload } from './index'
import type { DownloadAppFile } from 'types'

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
  uuidv4: vi.fn(() => 'test-uuid')
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
