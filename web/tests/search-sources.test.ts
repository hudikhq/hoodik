import { describe, it, expect, vi, beforeEach, beforeAll } from 'vitest'

/**
 * Create and rename must tag name and body as separate sources. Sending
 * `name` plus body as one list left stale body words on the name source after a
 * rename, and a save after a rename wiped the title. Api is mocked to capture
 * the body; tagging runs for real in WASM.
 */

vi.mock('../services/api', () => {
  const post = vi.fn()
  const put = vi.fn()
  return {
    default: class {
      constructor() {}
      toJson() {
        return {}
      }
      static post = post
      static put = put
    }
  }
})

import Api from '../services/api'
import * as meta from '../services/storage/meta'
import * as cryptfns from '../services/cryptfns'
import type { AppFile } from '../types'

const ApiPostMock = (Api as unknown as { post: ReturnType<typeof vi.fn> }).post
const ApiPutMock = (Api as unknown as { put: ReturnType<typeof vi.fn> }).put

describe('Search sources', () => {
  let kp: Awaited<ReturnType<typeof cryptfns.rsa.generateKeyPair>>

  beforeAll(async () => {
    kp = await cryptfns.rsa.generateKeyPair()
  })

  beforeEach(() => {
    ApiPostMock.mockReset()
    ApiPutMock.mockReset()
  })

  it('UNIT: create: name tokens are the title, content tokens are the body', async () => {
    const name = 'renewal.md'
    const body = 'The insurance policy renews in March.'
    let captured: Record<string, unknown> | undefined

    ApiPostMock.mockImplementation(async (_path: string, _params: unknown, payload: Record<string, unknown>) => {
      captured = payload
      throw new Error('stop after capture')
    })

    await expect(
      meta.create(kp, {
        name,
        mime: 'text/markdown',
        size: body.length,
        chunks: 1,
        content: body
      })
    ).rejects.toThrow('stop after capture')

    expect(captured).toBeDefined()
    const rootKey = cryptfns.searchRootKey(kp)
    expect(captured!.search_tokens_root).toEqual(cryptfns.searchTags(rootKey, name.toLowerCase()))
    expect(captured!.content_tokens_root).toEqual(cryptfns.searchTags(rootKey, body))

    const tags = JSON.stringify([
      captured!.search_tokens_root,
      captured!.search_tokens_file,
      captured!.content_tokens_root,
      captured!.content_tokens_file
    ])
    expect(tags).not.toContain('insurance')
    expect(tags).not.toContain('renewal')
  })

  it('UNIT: create: a binary has no content tokens', async () => {
    let captured: Record<string, unknown> | undefined

    ApiPostMock.mockImplementation(async (_path: string, _params: unknown, payload: Record<string, unknown>) => {
      captured = payload
      throw new Error('stop after capture')
    })

    await expect(
      meta.create(kp, {
        name: 'photo.jpg',
        mime: 'image/jpeg',
        size: 12,
        chunks: 1
      })
    ).rejects.toThrow('stop after capture')

    expect(captured!.content_tokens_root).toBeUndefined()
    expect(captured!.content_tokens_file).toBeUndefined()
    expect(captured!.search_tokens_root).toEqual(
      cryptfns.searchTags(cryptfns.searchRootKey(kp), 'photo.jpg')
    )
  })

  it('UNIT: rename: tags only the new name', async () => {
    const fileKey = await cryptfns.cipher.generateKey(cryptfns.cipher.defaultCipher())
    const file = {
      id: 'file-1',
      is_owner: true,
      key: fileKey,
      name: 'old.md',
      mime: 'text/markdown',
      encrypted_key: 'unused',
      encrypted_name: 'unused'
    } as unknown as AppFile

    let captured: Record<string, unknown> | undefined
    ApiPutMock.mockImplementation(async (_path: string, _params: unknown, payload: Record<string, unknown>) => {
      captured = payload
      throw new Error('stop after capture')
    })

    await expect(meta.rename(kp, file, { name: 'new-title.md' })).rejects.toThrow(
      'stop after capture'
    )

    const rootKey = cryptfns.searchRootKey(kp)
    expect(captured!.search_tokens_root).toEqual(cryptfns.searchTags(rootKey, 'new-title.md'))
    expect(captured!.content_tokens_root).toBeUndefined()
    expect(JSON.stringify([captured!.search_tokens_root, captured!.search_tokens_file])).not.toContain(
      'new-title'
    )
  })
})
