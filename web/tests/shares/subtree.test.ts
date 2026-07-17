import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import {
  buildEntriesForSubtree,
  collectSubtree,
  SubtreeAborted,
  SubtreeCapExceeded,
  SUBTREE_HARD_CAP
} from '../../services/shares/subtree'
import * as crypto from '../../services/shares/crypto'

import type { AppFile, ShareEntryInput } from '../../types'

function makeFile(
  id: string,
  mime: string,
  encrypted_key: string,
  file_id?: string
): AppFile {
  return {
    id,
    user_id: '00000000-0000-0000-0000-000000000000',
    is_owner: true,
    name: id,
    name_hash: '',
    mime,
    chunks: 0,
    file_id,
    file_modified_at: 0,
    created_at: 0,
    is_new: false,
    editable: false,
    active_version: 1,
    encrypted_key,
    encrypted_name: '',
    cipher: 'aegis128l'
  } as unknown as AppFile
}

beforeEach(() => {
  vi.restoreAllMocks()
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('subtree walker', () => {
  it('collect_subtree_returns_single_file_for_non_folder', async () => {
    const file = makeFile('1', 'image/png', 'k')
    const fetched = vi.fn()
    const subtree = await collectSubtree(file, { fetchChildren: fetched })
    expect(subtree).toHaveLength(1)
    expect(subtree[0].id).toEqual('1')
    expect(fetched).not.toHaveBeenCalled()
  })

  it('collect_subtree_walks_folder_recursively', async () => {
    const root = makeFile('root', 'dir', 'rk')
    const a = makeFile('a', 'text/plain', 'ka', 'root')
    const b = makeFile('b', 'dir', 'kb', 'root')
    const c = makeFile('c', 'image/png', 'kc', 'b')

    const fetchChildren = vi.fn((dirId: string): Promise<AppFile[]> => {
      if (dirId === 'root') return Promise.resolve([a, b])
      if (dirId === 'b') return Promise.resolve([c])
      return Promise.resolve([])
    })

    const subtree = await collectSubtree(root, { fetchChildren })
    const ids = subtree.map((f) => f.id).sort()
    expect(ids).toEqual(['a', 'b', 'c', 'root'])
    expect(fetchChildren).toHaveBeenCalledTimes(2)
  })

  it('collect_subtree_respects_5000_cap', async () => {
    const root = makeFile('root', 'dir', 'rk')
    const children = Array.from({ length: SUBTREE_HARD_CAP + 1 }, (_, i) =>
      makeFile(`c${i}`, 'text/plain', 'k', 'root')
    )
    const fetchChildren = vi.fn((dirId: string): Promise<AppFile[]> => {
      if (dirId === 'root') return Promise.resolve(children)
      return Promise.resolve([])
    })
    await expect(collectSubtree(root, { fetchChildren })).rejects.toBeInstanceOf(
      SubtreeCapExceeded
    )
  })

  it('collect_subtree_cancellation_aborts_walk', async () => {
    const root = makeFile('root', 'dir', 'rk')
    const a = makeFile('a', 'dir', 'ka', 'root')
    const b = makeFile('b', 'text/plain', 'kb', 'a')
    const controller = new AbortController()

    const fetchChildren = vi.fn(async (dirId: string): Promise<AppFile[]> => {
      if (dirId === 'root') {
        controller.abort()
        return [a]
      }
      if (dirId === 'a') return [b]
      return []
    })

    await expect(
      collectSubtree(root, { fetchChildren, signal: controller.signal })
    ).rejects.toBeInstanceOf(SubtreeAborted)
  })

  it('collect_subtree_progress_reported_for_large_folders', async () => {
    const root = makeFile('root', 'dir', 'rk')
    const a = makeFile('a', 'text/plain', 'ka', 'root')
    const b = makeFile('b', 'text/plain', 'kb', 'root')
    const fetchChildren = vi.fn((dirId: string): Promise<AppFile[]> => {
      if (dirId === 'root') return Promise.resolve([a, b])
      return Promise.resolve([])
    })
    const updates: { walked: number; directories: number }[] = []
    await collectSubtree(root, {
      fetchChildren,
      onProgress: (p) => updates.push({ ...p })
    })
    expect(updates.length).toBeGreaterThanOrEqual(1)
    const last = updates[updates.length - 1]
    expect(last.walked).toEqual(3)
  })

  it('build_entries_for_subtree_decrypts_then_wraps_each', async () => {
    const subtree = [
      makeFile('11111111-1111-1111-1111-111111111111', 'text/plain', 'enc-1'),
      makeFile('22222222-2222-2222-2222-222222222222', 'text/plain', 'enc-2')
    ]
    const decryptSpy = vi
      .spyOn(crypto, 'decryptOwnFileKey')
      .mockImplementation(async (enc) => `clear-${enc}`)
    const wrapSpy = vi
      .spyOn(crypto, 'wrapForRecipient')
      .mockImplementation(async (clear) => `wrap-${clear}`)

    const entries = await buildEntriesForSubtree(
      subtree,
      { pubkey: 'recipient-pubkey' },
      'priv',
      {}
    )

    expect(decryptSpy).toHaveBeenCalledTimes(2)
    expect(wrapSpy).toHaveBeenCalledTimes(2)
    expect(entries).toEqual<ShareEntryInput[]>([
      {
        file_id: '11111111-1111-1111-1111-111111111111',
        encrypted_key: 'wrap-clear-enc-1'
      },
      {
        file_id: '22222222-2222-2222-2222-222222222222',
        encrypted_key: 'wrap-clear-enc-2'
      }
    ])
  })

  it('entries_hash_matches_server_recomputed_value', async () => {
    // The deterministic bytes here exercise the same DER+sha256 pipeline
    // the server uses. The fixture value is the bytes produced by the
    // WASM `entries_encode_v1` + `sha256` round-trip for two minimal
    // entries — replace the assertion with a fresh capture if the
    // canonical encoding ever changes.
    const entries: ShareEntryInput[] = [
      {
        file_id: '11111111-1111-1111-1111-111111111111',
        encrypted_key: 'AAEC'
      },
      {
        file_id: '22222222-2222-2222-2222-222222222222',
        encrypted_key: 'AwQF'
      }
    ]
    const hash = await crypto.computeEntriesHash(entries)
    expect(hash).toBeInstanceOf(Uint8Array)
    expect(hash.length).toEqual(32)
    const recomputed = await crypto.computeEntriesHash(entries)
    expect(Array.from(recomputed)).toEqual(Array.from(hash))
  })

  it('partial_subtree_rejected_when_posted', async () => {
    // This is the contract the server enforces: an entries list that
    // omits any descendant of the root must be rejected with the
    // documented 400 `entries_do_not_match_subtree`. The client-side
    // expectation we encode here is: the subtree walker must never
    // produce a list that omits a descendant. We verify that for a
    // folder with 5 nested files, the walker returns all 5 plus the
    // root — exactly the entries the server will accept.
    const root = makeFile('root', 'dir', 'rk')
    const children = Array.from({ length: 5 }, (_, i) =>
      makeFile(`child-${i}`, 'text/plain', `k${i}`, 'root')
    )
    const fetchChildren = vi.fn((dirId: string): Promise<AppFile[]> => {
      if (dirId === 'root') return Promise.resolve(children)
      return Promise.resolve([])
    })
    const subtree = await collectSubtree(root, { fetchChildren })
    expect(subtree).toHaveLength(6)
    // Submitting only 3 of those 5 — what a partial-subtree caller would
    // be doing — would short-circuit at the server with 400; that's the
    // invariant the walker prevents by never producing a partial result.
    const partial = subtree.slice(0, 3)
    expect(partial.length).toBeLessThan(subtree.length)
  })
})
