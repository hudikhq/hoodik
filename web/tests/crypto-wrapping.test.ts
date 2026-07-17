import { describe, it, expect } from 'vitest'
import * as crypto from '../services/cryptfns'

describe('Wrapping test', () => {
  it('UNIT: Wrapping: can wrap and unwrap a file key', async () => {
    const privateKey = await crypto.wrapping.generatePrivateKey()
    const publicKey = await crypto.wrapping.publicFromPrivate(privateKey)
    const fileKey = await crypto.cipher.generateKey()

    const blob = await crypto.wrapping.wrap(fileKey, publicKey)
    const unwrapped = await crypto.wrapping.unwrap(blob, privateKey)

    expect(unwrapped).toEqual(fileKey)
  })

  it('UNIT: Wrapping: can wrap and unwrap an empty payload', async () => {
    const privateKey = await crypto.wrapping.generatePrivateKey()
    const publicKey = await crypto.wrapping.publicFromPrivate(privateKey)
    const payload = new Uint8Array(0)

    const blob = await crypto.wrapping.wrap(payload, publicKey)
    const unwrapped = await crypto.wrapping.unwrap(blob, privateKey)

    expect(unwrapped).toEqual(payload)
  })

  it('UNIT: Wrapping: can wrap and unwrap multi-byte characters', async () => {
    const privateKey = await crypto.wrapping.generatePrivateKey()
    const publicKey = await crypto.wrapping.publicFromPrivate(privateKey)
    const payload = crypto.uint8.fromUtf8('あいうえお')

    const blob = await crypto.wrapping.wrap(payload, publicKey)
    const unwrapped = await crypto.wrapping.unwrap(blob, privateKey)

    expect(crypto.uint8.toUtf8(unwrapped)).toBe('あいうえお')
  })

  it('UNIT: Wrapping: unwrap with the wrong private key fails', async () => {
    const privateKey = await crypto.wrapping.generatePrivateKey()
    const publicKey = await crypto.wrapping.publicFromPrivate(privateKey)
    const wrongPrivateKey = await crypto.wrapping.generatePrivateKey()
    const fileKey = await crypto.cipher.generateKey()

    const blob = await crypto.wrapping.wrap(fileKey, publicKey)

    await expect(crypto.wrapping.unwrap(blob, wrongPrivateKey)).rejects.toThrow()
  })

  it('UNIT: Wrapping: wrapping the same key twice produces different blobs', async () => {
    const privateKey = await crypto.wrapping.generatePrivateKey()
    const publicKey = await crypto.wrapping.publicFromPrivate(privateKey)
    const fileKey = await crypto.cipher.generateKey()

    const first = await crypto.wrapping.wrap(fileKey, publicKey)
    const second = await crypto.wrapping.wrap(fileKey, publicKey)

    expect(first).not.toBe(second)
    expect(await crypto.wrapping.unwrap(first, privateKey)).toEqual(fileKey)
    expect(await crypto.wrapping.unwrap(second, privateKey)).toEqual(fileKey)
  })
})
