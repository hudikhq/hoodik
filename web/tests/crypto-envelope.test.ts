import { describe, it, expect } from 'vitest'
import * as crypto from '../services/cryptfns'

function randomExportKey(): Uint8Array {
  const bytes = new Uint8Array(64)
  globalThis.crypto.getRandomValues(bytes)
  return bytes
}

describe('Envelope test', () => {
  it('UNIT: Envelope: deriveKek is deterministic for the same export key', async () => {
    const exportKey = randomExportKey()

    const first = await crypto.envelope.deriveKek(exportKey)
    const second = await crypto.envelope.deriveKek(exportKey)

    expect(first.length).toBe(32)
    expect(first).toEqual(second)
  })

  it('UNIT: Envelope: different export keys derive different keks', async () => {
    const first = await crypto.envelope.deriveKek(randomExportKey())
    const second = await crypto.envelope.deriveKek(randomExportKey())

    expect(first).not.toEqual(second)
  })

  it('UNIT: Envelope: seal then open round-trips an empty bundle', async () => {
    const kek = await crypto.envelope.deriveKek(randomExportKey())
    const bundle = new Uint8Array(0)

    const envelope = await crypto.envelope.seal(kek, bundle)
    const opened = await crypto.envelope.open(kek, envelope)

    expect(opened).toEqual(bundle)
  })

  it('UNIT: Envelope: seal then open round-trips a small bundle', async () => {
    const kek = await crypto.envelope.deriveKek(randomExportKey())
    const bundle = crypto.uint8.fromUtf8('あいうえお')

    const envelope = await crypto.envelope.seal(kek, bundle)
    const opened = await crypto.envelope.open(kek, envelope)

    expect(opened).toEqual(bundle)
  })

  it('UNIT: Envelope: seal then open round-trips a large bundle', async () => {
    const kek = await crypto.envelope.deriveKek(randomExportKey())
    const bundle = new Uint8Array(64 * 1024)
    globalThis.crypto.getRandomValues(bundle)

    const envelope = await crypto.envelope.seal(kek, bundle)
    const opened = await crypto.envelope.open(kek, envelope)

    expect(opened).toEqual(bundle)
  })

  it('UNIT: Envelope: opening with the wrong kek fails', async () => {
    const kek = await crypto.envelope.deriveKek(randomExportKey())
    const wrongKek = await crypto.envelope.deriveKek(randomExportKey())
    const bundle = crypto.uint8.fromUtf8('secret bundle')

    const envelope = await crypto.envelope.seal(kek, bundle)

    await expect(crypto.envelope.open(wrongKek, envelope)).rejects.toThrow()
  })

  it('UNIT: Envelope: rewrap re-keys an envelope to a new kek', async () => {
    const kekA = await crypto.envelope.deriveKek(randomExportKey())
    const kekB = await crypto.envelope.deriveKek(randomExportKey())
    const bundle = crypto.uint8.fromUtf8('secret bundle')

    const sealed = await crypto.envelope.seal(kekA, bundle)
    const rewrapped = await crypto.envelope.rewrap(kekA, kekB, sealed)

    expect(await crypto.envelope.open(kekB, rewrapped)).toEqual(bundle)
    await expect(crypto.envelope.open(kekA, rewrapped)).rejects.toThrow()
  })
})
