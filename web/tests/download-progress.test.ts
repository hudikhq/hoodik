import { afterEach, describe, expect, it, vi } from 'vitest'

import * as cryptfns from '../services/cryptfns'
import { downloadAndDecrypt } from '../services/storage/download/sync'
import type { AppFile } from '../types'

/**
 * Serves one encrypted chunk as a streaming Response body delivered in
 * pieces, the way a slow network would. The download runs through the real
 * wasm pipeline — HTTP, byte tally, decryption and ordering all in the
 * crate — so what these tests pin is the actual wire-to-callback path.
 */
function streamingFetch(requested: string[], pieces: () => Uint8Array[]) {
  return async (input: RequestInfo | URL) => {
    requested.push(String(input instanceof Request ? input.url : input))

    return new Response(
      new ReadableStream({
        start(controller) {
          pieces().forEach((piece) => controller.enqueue(piece))
          controller.close()
        }
      }),
      { status: 200 }
    )
  }
}

function split(data: Uint8Array): Uint8Array[] {
  const half = Math.floor(data.length / 2)
  return [data.slice(0, half), data.slice(half)]
}

describe('download byte progress through the wasm pipeline', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  async function makeFile(plaintext: Uint8Array) {
    const cipher = cryptfns.cipher.defaultCipher()
    const key = await cryptfns.cipher.generateKey(cipher)
    const encrypted = await cryptfns.cipher.encrypt(cipher, plaintext, key, 0)

    const file = {
      id: 'f1',
      chunks: 1,
      size: plaintext.length,
      cipher,
      key
    } as unknown as AppFile

    return { file, encrypted }
  }

  it('reports bytes before the chunk completes and decrypts correctly', async () => {
    const plaintext = new TextEncoder().encode('progressively downloaded plaintext')
    const { file, encrypted } = await makeFile(plaintext)

    const requested: string[] = []
    vi.stubGlobal('fetch', streamingFetch(requested, () => split(encrypted)))

    const reported: number[] = []
    const data = await downloadAndDecrypt(file, (bytes) => reported.push(bytes))

    expect(new TextDecoder().decode(data)).toBe('progressively downloaded plaintext')
    expect(requested).toHaveLength(1)
    expect(requested[0]).toContain(`/api/storage/${file.id}?chunk=0`)

    // Movement exists before completion, never regresses, and the final
    // event is the exact plaintext size.
    expect(reported.length).toBeGreaterThan(0)
    for (let i = 1; i < reported.length; i++) {
      expect(reported[i]).toBeGreaterThanOrEqual(reported[i - 1])
    }
    expect(reported[reported.length - 1]).toBe(plaintext.length)
  })

  it('still downloads when no observer is attached', async () => {
    const plaintext = new TextEncoder().encode('silent download')
    const { file, encrypted } = await makeFile(plaintext)

    vi.stubGlobal('fetch', streamingFetch([], () => split(encrypted)))

    const data = await downloadAndDecrypt(file)

    expect(new TextDecoder().decode(data)).toBe('silent download')
  })
})
