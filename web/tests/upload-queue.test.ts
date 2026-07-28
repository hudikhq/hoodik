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
import { store as uploadStore } from '../services/storage/upload'
import type { FilesStore, QueueStore, UploadAppFile } from '../types'

const storage = {
  dir: undefined,
  getItem: () => undefined,
  updateItem: () => undefined,
  upsertItem: () => undefined
} as unknown as FilesStore

const queue = { uploadWorkerListenerActive: true } as unknown as QueueStore

function makeFile(id: string): UploadAppFile {
  return {
    id,
    temporaryId: id,
    name: `${id}.bin`,
    mime: 'application/octet-stream',
    size: 1,
    chunks: 1,
    file_id: null
  } as unknown as UploadAppFile
}

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
    upload.waiting.push(makeFile('a'), makeFile('b'), makeFile('c'))

    interval = await upload.start(storage, queue)
    await vi.advanceTimersByTimeAsync(1000)

    expect(upload.running.map((f: UploadAppFile) => f.id)).toEqual(['a'])
    expect(upload.waiting.map((f: UploadAppFile) => f.id)).toEqual(['b', 'c'])
  })

  it('UNIT: the concurrency limit holds while the dispatched file is unacknowledged', async () => {
    const upload = uploadStore()
    upload.waiting.push(makeFile('a'), makeFile('b'), makeFile('c'))

    interval = await upload.start(storage, queue)
    await vi.advanceTimersByTimeAsync(3000)

    expect(upload.running).toHaveLength(1)
    expect(upload.waiting).toHaveLength(2)
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
