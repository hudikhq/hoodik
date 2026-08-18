import { describe, it, expect, vi, beforeEach } from 'vitest'

/**
 * The privacy contract for search: the typed query is tokenized and tagged
 * inside the browser under a key the server never sees, and only those tags
 * go over the wire. Api is mocked to capture the body; the tokenizer and the
 * tagging run for real in WASM, so the assertions cover what is actually sent.
 *
 * The bare-digest assertion is the one that matters most. The index used to
 * store unsalted SHA-256 of each token, which a table over the public BERT
 * vocabulary reverses in seconds — so it is not enough that the plaintext is
 * absent, its digest has to be absent too.
 */

vi.mock('../services/api', () => {
  const post = vi.fn()
  return {
    default: class {
      constructor() {}
      toJson() {
        return {}
      }
      static post = post
    }
  }
})

import Api from '../services/api'
import * as meta from '../services/storage/meta'
import * as cryptfns from '../services/cryptfns'
import type { KeyPair } from '../types'

/** A throwaway account key; only its stability across a test matters. */
const keypair = {
  input: null,
  publicKey: null,
  fingerprint: null,
  keySize: 0,
  wrappingPrivate: cryptfns.wasm.wrapping_generate_private()
} as unknown as KeyPair

const ApiPostMock = (Api as unknown as { post: ReturnType<typeof vi.fn> }).post

describe('Search privacy', () => {
  beforeEach(() => {
    ApiPostMock.mockReset()
    ApiPostMock.mockResolvedValue({ body: [] })
  })

  it('UNIT: search: sends only hashed tokens, never the plaintext query', async () => {
    await meta.search('Annual Report', keypair)

    expect(ApiPostMock).toHaveBeenCalledTimes(1)
    const [path, params, body] = ApiPostMock.mock.calls[0]

    expect(path).toBe('/api/storage/search')
    expect(params).toBeUndefined()
    expect(body).not.toHaveProperty('search')
    // An ordinary query is not a digest, so nothing goes over verbatim.
    expect(body.hash).toBeUndefined()

    // Tagging matches the upload path, which indexes the lowercased name.
    const rootKey = cryptfns.searchRootKey(keypair)
    expect(body.root_tags).toEqual(
      cryptfns.searchTags(rootKey, 'annual report').map((t: string) => t.split(':')[0])
    )
    expect(body.root_tags.length).toBeGreaterThan(0)
    for (const tag of body.root_tags) {
      expect(tag).toMatch(/^[0-9a-f]{32}$/)
    }

    // Nothing owned means no per-file tags to send.
    expect(body.file_tags).toEqual([])

    // "annual" and "report" contain non-hex letters, so neither can hide
    // inside a hex tag — absence here proves the plaintext stayed home.
    const wire = JSON.stringify(body).toLowerCase()
    expect(wire).not.toContain('annual')
    expect(wire).not.toContain('report')

    // And the old scheme's digests must not appear either: an index keyed on
    // these is the thing this change exists to remove.
    for (const word of ['annual', 'report']) {
      expect(wire).not.toContain(cryptfns.sha256.digest(word))
    }
  })

  it('UNIT: search: a content digest goes over verbatim as a hash lookup', async () => {
    // Every digest length the file rows carry: MD5, SHA1, SHA256, BLAKE2b.
    for (const length of [32, 40, 64, 128]) {
      ApiPostMock.mockClear()
      const digest = 'f'.repeat(length)

      await meta.search(digest, keypair)

      const [, , body] = ApiPostMock.mock.calls[0]
      expect(body.hash).toBe(digest)
      expect(body).not.toHaveProperty('search')
    }
  })

  it('UNIT: search: near-digest strings are not treated as hash lookups', async () => {
    // One character short of SHA256, and a same-length string with a
    // non-hex character in it.
    for (const candidate of ['f'.repeat(63), `g${'f'.repeat(63)}`]) {
      ApiPostMock.mockClear()

      await meta.search(candidate, keypair)

      const [, , body] = ApiPostMock.mock.calls[0]
      expect(body.hash).toBeUndefined()
    }
  })

  it('UNIT: search: options are forwarded alongside the hashed tokens', async () => {
    await meta.search('budget', keypair, [], { dir_id: 'dir-1', editable: true, limit: 50 })

    const [, , body] = ApiPostMock.mock.calls[0]
    expect(body.dir_id).toBe('dir-1')
    expect(body.editable).toBe(true)
    expect(body.limit).toBe(50)
    expect(body.skip).toBe(0)
  })
})
