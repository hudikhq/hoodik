import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'

import { handleHashDoneMessage } from '../services/queue'
import * as meta from '../services/storage/meta'

import type { AppFile, FilesStore } from '../types'

/**
 * The moment the digest lands is also the moment the upload usually finishes,
 * and the two race: the finish refresh rewrites the store row while the
 * hashes PUT is in flight. Writing the pre-PUT snapshot back would strip
 * `finished_upload_at` from the row the user is looking at — a file stuck
 * showing the half-uploaded action set until the next full refresh, which is
 * exactly how a run of share/download/rename E2Es once went red at once.
 */

// The rows below carry `is_owner: false`, so the root-scope branch — the one
// that needs real key material — never runs; it is exercised end-to-end by
// the digest search E2E. These tests are about the store handling around the
// PUT. The factory runs lazily on the handler's dynamic import, after
// `keypair` below is initialised.
let keypair: Record<string, unknown> = {}
vi.mock('../services/crypto', () => ({
  store: () => ({ keypair })
}))

const digest = 'c'.repeat(64)

function row(overrides: Partial<AppFile> = {}): AppFile {
  return {
    id: 'file-1',
    name: 'holiday.mov',
    key: new Uint8Array(32),
    is_owner: false,
    ...overrides
  } as unknown as AppFile
}

function storeWith(rows: (AppFile | undefined)[]): FilesStore & { updated: AppFile[] } {
  const updated: AppFile[] = []
  let call = 0
  return {
    getItem: () => rows[Math.min(call++, rows.length - 1)],
    updateItem: (item: AppFile) => updated.push(item),
    updated
  } as unknown as FilesStore & { updated: AppFile[] }
}

describe('hash-done handling', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.restoreAllMocks()
    keypair = { input: 'unlocked' }
  })

  it('re-reads the row after the PUT so a finish landing mid-flight survives', async () => {
    const finished = row({ finished_upload_at: '2026-08-22T10:00:00Z' })
    const files = storeWith([row(), finished])
    const put = vi.spyOn(meta, 'updateHashes').mockResolvedValue(finished)

    await handleHashDoneMessage(files, 'file-1', digest)

    expect(put).toHaveBeenCalledOnce()
    const [, update] = put.mock.calls[0]
    // Keyed, never the bare digest.
    expect(update.sha256).toMatch(/^[0-9a-f]{32}$/)
    expect(update.sha256).not.toBe(digest)

    expect(files.updated).toHaveLength(1)
    expect(files.updated[0].finished_upload_at).toBe('2026-08-22T10:00:00Z')
    expect(files.updated[0].sha256).toBe(update.sha256)
  })

  it('fetches the row back when navigation evicted it from the store', async () => {
    const files = storeWith([undefined])
    const put = vi.spyOn(meta, 'updateHashes').mockResolvedValue(row())
    vi.spyOn(meta, 'get').mockResolvedValue(row())

    await handleHashDoneMessage(files, 'file-1', digest)

    expect(put).toHaveBeenCalledOnce()
    // The store has moved on; there is no row to write back to.
    expect(files.updated).toHaveLength(0)
  })

  it('gives up without a request when neither the store nor the keypair can key it', async () => {
    keypair = {}
    const files = storeWith([undefined])
    const put = vi.spyOn(meta, 'updateHashes')

    await handleHashDoneMessage(files, 'file-1', digest)

    expect(put).not.toHaveBeenCalled()
  })
})
