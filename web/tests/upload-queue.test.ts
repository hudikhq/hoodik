import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'

// The dispatch never settles — it stands in for the transfer-token round
// trip the worker makes before it acknowledges the file it was handed.
vi.mock('../services/storage/workers', () => ({
  pushUploadToWorker: () => new Promise<void>(() => undefined),
  startFileDownload: () => new Promise<void>(() => undefined)
}))

import StatusBar from '../src/components/files/io/StatusBar.vue'
import { store as uploadStore, takeDispatchable } from '../services/storage/upload'
import { CONCURRENT_CHUNKS_UPLOAD } from '../services/constants'
import type { FilesStore, QueueStore, UploadAppFile } from '../types'

const storage = {
  dir: undefined,
  getItem: () => undefined,
  updateItem: () => undefined,
  upsertItem: () => undefined
} as unknown as FilesStore

const queue = { uploadWorkerListenerActive: true } as unknown as QueueStore

function makeFile(id: string, chunks = 1, chunks_stored = 0): UploadAppFile {
  return {
    id,
    temporaryId: id,
    name: `${id}.bin`,
    mime: 'application/octet-stream',
    size: 1,
    chunks,
    chunks_stored,
    file_id: null
  } as unknown as UploadAppFile
}

const ids = (files: UploadAppFile[]) => files.map((f) => f.id)

describe('takeDispatchable', () => {
  it('UNIT: small files run side by side up to the chunk budget', () => {
    const waiting = Array.from({ length: CONCURRENT_CHUNKS_UPLOAD + 2 }, (_, i) =>
      makeFile(`f${i}`)
    )

    const taken = takeDispatchable(waiting, [])

    expect(taken).toHaveLength(CONCURRENT_CHUNKS_UPLOAD)
    expect(waiting).toHaveLength(2)
  })

  it('UNIT: a large file runs alone', () => {
    const waiting = [makeFile('big', 100), makeFile('small')]

    expect(ids(takeDispatchable(waiting, []))).toEqual(['big'])
    expect(ids(takeDispatchable(waiting, [makeFile('big', 100)]))).toEqual([])
  })

  it('UNIT: small files do not overtake a large one at the head', () => {
    const waiting = [makeFile('a'), makeFile('big', 100), makeFile('b')]

    expect(ids(takeDispatchable(waiting, []))).toEqual(['a'])
    expect(ids(waiting)).toEqual(['big', 'b'])
  })

  it('UNIT: small files join a large file once few of its chunks are left', () => {
    const running = [makeFile('big', 100, 98)]
    const waiting = Array.from({ length: CONCURRENT_CHUNKS_UPLOAD }, (_, i) => makeFile(`f${i}`))

    expect(takeDispatchable(waiting, running)).toHaveLength(CONCURRENT_CHUNKS_UPLOAD - 2)
  })

  it('UNIT: an empty file still takes a slot', () => {
    const waiting = [makeFile('empty', 0)]

    expect(ids(takeDispatchable(waiting, []))).toEqual(['empty'])
  })
})

describe('upload queue dispatch', () => {
  let interval: ReturnType<typeof setInterval>

  beforeEach(() => {
    setActivePinia(createPinia())
    vi.useFakeTimers()
  })

  afterEach(() => {
    clearInterval(interval)
    vi.useRealTimers()
  })

  it('UNIT: a dispatched file is running before the worker acknowledges it', async () => {
    const upload = uploadStore()
    upload.waiting.push(makeFile('a', 100), makeFile('b'), makeFile('c'))

    interval = await upload.start(storage, queue)
    await vi.advanceTimersByTimeAsync(1000)

    expect(ids(upload.running)).toEqual(['a'])
    expect(ids(upload.waiting)).toEqual(['b', 'c'])
  })

  it('UNIT: the concurrency limit holds while the dispatched file is unacknowledged', async () => {
    const upload = uploadStore()
    upload.waiting.push(makeFile('a', 100), makeFile('b'), makeFile('c'))

    interval = await upload.start(storage, queue)
    await vi.advanceTimersByTimeAsync(3000)

    expect(upload.running).toHaveLength(1)
    expect(upload.waiting).toHaveLength(2)
  })

  it('UNIT: a finished file hands its slot on without waiting for the interval', async () => {
    const upload = uploadStore()
    const big = makeFile('a', 100)
    upload.waiting.push(big, makeFile('b'))

    interval = await upload.start(storage, queue)
    await vi.advanceTimersByTimeAsync(1000)
    expect(ids(upload.running)).toEqual(['a'])

    await upload.progress(storage, big, true)
    await vi.advanceTimersByTimeAsync(0)

    expect(ids(upload.running)).toEqual(['b'])
  })

  it('UNIT: a failed file hands its slot on without waiting for the interval', async () => {
    const upload = uploadStore()
    const big = makeFile('a', 100)
    upload.waiting.push(big, makeFile('b'))

    interval = await upload.start(storage, queue)
    await vi.advanceTimersByTimeAsync(1000)

    await upload.progress(storage, big, false, { context: 'boom' })
    await vi.advanceTimersByTimeAsync(0)

    expect(ids(upload.running)).toEqual(['b'])
  })

  it('UNIT: the worker acknowledgement does not duplicate the running entry', async () => {
    const upload = uploadStore()
    const file = makeFile('a')
    upload.waiting.push(file)

    interval = await upload.start(storage, queue)
    await vi.advanceTimersByTimeAsync(1000)

    await upload.progress(storage, file, false)

    expect(upload.running.map((f: UploadAppFile) => f.id)).toEqual(['a'])
  })
})

describe('StatusBar transfer sentinel', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('UNIT: shows while a file is queued but not yet dispatched', () => {
    uploadStore().waiting.push(makeFile('a'))

    const wrapper = mount(StatusBar)

    expect(wrapper.find('[data-testid="upload-active"]').exists()).toBe(true)
  })

  it('UNIT: stays hidden when nothing is queued', () => {
    const wrapper = mount(StatusBar)

    expect(wrapper.find('[data-testid="upload-active"]').exists()).toBe(false)
  })
})
