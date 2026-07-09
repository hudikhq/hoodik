import { describe, it, expect } from 'vitest'
import * as crypto from '../services/cryptfns'

describe('X25519 test', () => {
  it('UNIT: X25519: can wrap and unwrap a file key', async () => {
    const privateKey = await crypto.x25519.generatePrivateKey()
    const publicKey = await crypto.x25519.publicFromPrivate(privateKey)
    const fileKey = await crypto.cipher.generateKey()

    const blob = await crypto.x25519.wrap(fileKey, publicKey)
    const unwrapped = await crypto.x25519.unwrap(blob, privateKey)

    expect(unwrapped).toEqual(fileKey)
  })

  it('UNIT: X25519: can wrap and unwrap an empty payload', async () => {
    const privateKey = await crypto.x25519.generatePrivateKey()
    const publicKey = await crypto.x25519.publicFromPrivate(privateKey)
    const payload = new Uint8Array(0)

    const blob = await crypto.x25519.wrap(payload, publicKey)
    const unwrapped = await crypto.x25519.unwrap(blob, privateKey)

    expect(unwrapped).toEqual(payload)
  })

  it('UNIT: X25519: can wrap and unwrap multi-byte characters', async () => {
    const privateKey = await crypto.x25519.generatePrivateKey()
    const publicKey = await crypto.x25519.publicFromPrivate(privateKey)
    const payload = crypto.uint8.fromUtf8('あいうえお')

    const blob = await crypto.x25519.wrap(payload, publicKey)
    const unwrapped = await crypto.x25519.unwrap(blob, privateKey)

    expect(crypto.uint8.toUtf8(unwrapped)).toBe('あいうえお')
  })

  it('UNIT: X25519: unwrap with the wrong private key fails', async () => {
    const privateKey = await crypto.x25519.generatePrivateKey()
    const publicKey = await crypto.x25519.publicFromPrivate(privateKey)
    const wrongPrivateKey = await crypto.x25519.generatePrivateKey()
    const fileKey = await crypto.cipher.generateKey()

    const blob = await crypto.x25519.wrap(fileKey, publicKey)

    await expect(crypto.x25519.unwrap(blob, wrongPrivateKey)).rejects.toThrow()
  })

  it('UNIT: X25519: wrapping the same key twice produces different blobs', async () => {
    const privateKey = await crypto.x25519.generatePrivateKey()
    const publicKey = await crypto.x25519.publicFromPrivate(privateKey)
    const fileKey = await crypto.cipher.generateKey()

    const first = await crypto.x25519.wrap(fileKey, publicKey)
    const second = await crypto.x25519.wrap(fileKey, publicKey)

    expect(first).not.toBe(second)
    expect(await crypto.x25519.unwrap(first, privateKey)).toEqual(fileKey)
    expect(await crypto.x25519.unwrap(second, privateKey)).toEqual(fileKey)
  })
})
