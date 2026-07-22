import { describe, it, expect, vi, beforeEach } from 'vitest'

/**
 * The privacy contract for search: the typed query is tokenized and
 * SHA-256 hashed inside the browser, and only those hashes go to the
 * server. Api is mocked to capture the wire body; the tokenizer runs
 * for real in WASM so the assertions cover the actual hashes sent.
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
import { stringToHashedTokens } from '../services/cryptfns'

const ApiPostMock = (Api as unknown as { post: ReturnType<typeof vi.fn> }).post

describe('Search privacy', () => {
  beforeEach(() => {
    ApiPostMock.mockReset()
    ApiPostMock.mockResolvedValue({ body: [] })
  })

  it('UNIT: search: sends only hashed tokens, never the plaintext query', async () => {
    await meta.search('Annual Report')

    expect(ApiPostMock).toHaveBeenCalledTimes(1)
    const [path, params, body] = ApiPostMock.mock.calls[0]

    expect(path).toBe('/api/storage/search')
    expect(params).toBeUndefined()
    expect(body).not.toHaveProperty('search')

    // Tokenization matches the upload path, which indexes the lowercased name.
    expect(body.search_tokens_hashed).toEqual(stringToHashedTokens('annual report'))
    expect(body.search_tokens_hashed.length).toBeGreaterThan(0)
    for (const token of body.search_tokens_hashed) {
      expect(token).toMatch(/^[0-9a-f]{64}:\d+$/)
    }

    // "annual" and "report" contain non-hex letters, so neither can hide
    // inside a hex digest — absence here proves the plaintext stayed home.
    const wire = JSON.stringify(body).toLowerCase()
    expect(wire).not.toContain('annual')
    expect(wire).not.toContain('report')
  })

  it('UNIT: search: a hash-shaped query is tokenized like any other input', async () => {
    const sha256 = 'f'.repeat(64)

    await meta.search(sha256)

    const [, , body] = ApiPostMock.mock.calls[0]
    expect(body).not.toHaveProperty('search')
    expect(body).not.toHaveProperty('hash')
    expect(body.search_tokens_hashed).toEqual(stringToHashedTokens(sha256))
  })

  it('UNIT: search: options are forwarded alongside the hashed tokens', async () => {
    await meta.search('budget', { dir_id: 'dir-1', editable: true, limit: 50 })

    const [, , body] = ApiPostMock.mock.calls[0]
    expect(body.dir_id).toBe('dir-1')
    expect(body.editable).toBe(true)
    expect(body.limit).toBe(50)
    expect(body.skip).toBe(0)
  })
})
