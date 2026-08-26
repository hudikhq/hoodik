import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import { store as uploadStore } from '../services/storage/upload'

import type { FilesStore, UploadAppFile } from '../types'

/**
 * What the queue does with a failure that arrives after the last chunk.
 *
 * On the direct path the commit is a request of its own, sent once every
 * chunk is in the bucket. So "every chunk landed" is not "the file is on the
 * server", and a failure can arrive for a file the UI is already showing as
 * finished. It has to be shown: the alternative is a user looking at an
 * uploaded file that the server never committed and cannot serve back.
 */

const storage = {
  dir: undefined,
  getItem: () => undefined,
  updateItem: () => undefined,
  upsertItem: () => undefined,
  removeItem: () => undefined
} as unknown as FilesStore

/** A listing that records what the queue does to it. */
function trackingStore() {
  const upserted: string[] = []
  const removed: string[] = []

  return {
    store: {
      dir: undefined,
      getItem: () => undefined,
      updateItem: () => undefined,
      upsertItem: (f: UploadAppFile) => upserted.push(f.id),
      removeItem: (id: string) => removed.push(id)
    } as unknown as FilesStore,
    upserted,
    removed
  }
}

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

describe('a commit that fails after the chunks landed', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('UNIT: moves the file out of done and into failed', async () => {
    const upload = uploadStore()
    const file = makeFile('a')

    await upload.progress(storage, file, true)
    expect(upload.done.map((f: UploadAppFile) => f.id)).toEqual(['a'])

    await upload.progress(storage, makeFile('a'), false, { context: 'finalize failed' })

    expect(upload.done.map((f: UploadAppFile) => f.id)).toEqual([])
    expect(upload.failed.map((f: UploadAppFile) => f.id)).toEqual(['a'])
  })

  it('UNIT: a late hash update still merges without disturbing a finished file', async () => {
    const upload = uploadStore()
    const file = makeFile('b')

    await upload.progress(storage, file, true)
    // The hash worker reports long after the upload finished; no error, so
    // nothing about the row's completion changes.
    await upload.progress(storage, { ...makeFile('b'), sha256: 'deadbeef' } as UploadAppFile, false)

    expect(upload.done.map((f: UploadAppFile) => f.id)).toEqual(['b'])
    expect(upload.failed).toHaveLength(0)
  })

  it('UNIT: a file is never listed as both done and failed', async () => {
    const upload = uploadStore()

    await upload.progress(storage, makeFile('c'), true)
    await upload.progress(storage, makeFile('c'), false, { context: 'finalize failed' })

    const inBoth = upload.done.filter((d: UploadAppFile) =>
      upload.failed.some((f: UploadAppFile) => f.id === d.id)
    )
    expect(inBoth).toEqual([])
  })

  it('UNIT: a report arriving after cancel does not put the row back', async () => {
    const upload = uploadStore()
    const file = makeFile('d')
    const listing = trackingStore()

    upload.running.push(file)
    // The delete goes to the server inside `cancel`; here only the queue's
    // own bookkeeping is under test.
    await upload.cancel(listing.store, file).catch(() => undefined)

    // What the worker sends next: its own copy of the file, which never
    // learned about the cancel, reporting the chunk that was already in
    // flight.
    await upload.progress(listing.store, makeFile('d'), false)

    expect(listing.upserted).not.toContain('d')
    expect(upload.running.map((f: UploadAppFile) => f.id)).toEqual([])
    expect(upload.failed.map((f: UploadAppFile) => f.id)).toEqual(['d'])
  })
})
