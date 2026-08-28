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
    // The retired verbatim hash field must never come back.
    expect(body.hash).toBeUndefined()

    // Tagging matches the upload path, which indexes the lowercased name —
    // plus one exact-match tag of the whole query, which is what answers a
    // pasted content digest without any of it crossing in plaintext.
    const rootKey = cryptfns.searchRootKey(keypair)
    expect(body.root_tags).toEqual([
      ...cryptfns.searchTags(rootKey, 'annual report').map((t: string) => t.split(':')[0]),
      cryptfns.searchTag(rootKey, 'annual report')
    ])
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

  it('UNIT: search: a pasted digest goes over as a keyed exact-match tag, never verbatim', async () => {
    // Every digest length the file rows carry: MD5, SHA1, SHA256, BLAKE2b —
    // and there is nothing special about them any more: any query gets one
    // exact-match tag, and a digest is findable because indexing tagged it
    // the same way when the hashes landed.
    for (const length of [32, 40, 64, 128]) {
      ApiPostMock.mockClear()
      const digest = 'f'.repeat(length)

      await meta.search(digest, keypair)

      const [, , body] = ApiPostMock.mock.calls[0]
      expect(body.hash).toBeUndefined()
      expect(body).not.toHaveProperty('search')

      const rootKey = cryptfns.searchRootKey(keypair)
      expect(body.root_tags).toContain(cryptfns.searchTag(rootKey, digest))
      expect(JSON.stringify(body)).not.toContain(digest)
    }
  })

  it('UNIT: search: a capitalized word is findable by a lowercase query', async () => {
    // Every query is lowercased, and the tokenizer is cased, so tagging folds
    // case — otherwise a note body saved as written never matches.
    const rootKey = cryptfns.searchRootKey(keypair)

    expect(cryptfns.searchTags(rootKey, 'Berlin Meetup')).toEqual(
      cryptfns.searchTags(rootKey, 'berlin meetup')
    )
  })

  it('UNIT: search: matches the pinned cross-client tag vector', () => {
    // The same key and input pinned in the server's cryptfns suite and in the
    // app's. Web tags through WASM and the app through FFI, both over the same
    // crate — a drift in tokenization, case folding or the tag scheme splits
    // an account's index between its clients. This fails first instead.
    // Regenerate all three together on a deliberate change.
    const keyHex = Array.from({ length: 32 }, (_, i) => i.toString(16).padStart(2, '0')).join('')

    expect(cryptfns.searchTags(keyHex, 'Invoice Q1')).toEqual([
      'ec4767d0aabcccd2fc223bf3afde7a6c:1',
      'bac924ee3ab4879a38f37ee48077cc3f:1'
    ])
  })

  it('UNIT: search: options are forwarded alongside the hashed tokens', async () => {
    await meta.search('budget', keypair, [], { dir_id: 'dir-1', editable: true, limit: 50 })

    const [, , body] = ApiPostMock.mock.calls[0]
    expect(body.dir_id).toBe('dir-1')
    expect(body.editable).toBe(true)
    expect(body.limit).toBe(50)
    expect(body.skip).toBe(0)

    // The exact-name fast path: the raw trimmed query hashed the way create
    // hashes names, so the server can rank a pasted filename first.
    expect(body.name_hash).toBe(cryptfns.searchTag(cryptfns.searchRootKey(keypair), 'budget'))
  })
})
