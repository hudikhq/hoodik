import { describe, it, expect } from 'vitest'
import * as crypto from '../services/cryptfns'

describe('Ed25519 test', () => {
  it('UNIT: ED25519: can sign a message and verify the signature', async () => {
    const privateKey = await crypto.ed25519.generatePrivateKey()
    const publicKey = await crypto.ed25519.publicFromPrivate(privateKey)

    const message = 'hello world'
    const signature = await crypto.ed25519.sign(message, privateKey)

    expect(await crypto.ed25519.verify(message, signature, publicKey)).toBe(true)
  })

  it('UNIT: ED25519: can sign raw bytes and verify the signature', async () => {
    const privateKey = await crypto.ed25519.generatePrivateKey()
    const publicKey = await crypto.ed25519.publicFromPrivate(privateKey)

    const message = new Uint8Array([0, 255, 128, 1, 2, 3])
    const signature = await crypto.ed25519.signBytes(message, privateKey)

    expect(await crypto.ed25519.verifyBytes(message, signature, publicKey)).toBe(true)
  })

  it('UNIT: ED25519: verifying a tampered message fails', async () => {
    const privateKey = await crypto.ed25519.generatePrivateKey()
    const publicKey = await crypto.ed25519.publicFromPrivate(privateKey)

    const signature = await crypto.ed25519.sign('hello world', privateKey)

    expect(await crypto.ed25519.verify('hello w0rld', signature, publicKey)).toBe(false)

    const bytes = new Uint8Array([1, 2, 3])
    const bytesSignature = await crypto.ed25519.signBytes(bytes, privateKey)

    expect(
      await crypto.ed25519.verifyBytes(new Uint8Array([1, 2, 4]), bytesSignature, publicKey)
    ).toBe(false)
  })

  it('UNIT: ED25519: verifying with the wrong public key fails', async () => {
    const privateKey = await crypto.ed25519.generatePrivateKey()
    const wrongPublicKey = await crypto.ed25519.publicFromPrivate(
      await crypto.ed25519.generatePrivateKey()
    )

    const message = 'hello world'
    const signature = await crypto.ed25519.sign(message, privateKey)

    expect(await crypto.ed25519.verify(message, signature, wrongPublicKey)).toBe(false)
  })

  it('UNIT: ED25519: fingerprint is 64 hex characters and deterministic', async () => {
    const privateKey = await crypto.ed25519.generatePrivateKey()
    const publicKey = await crypto.ed25519.publicFromPrivate(privateKey)

    const fingerprint = await crypto.ed25519.fingerprint(publicKey)

    expect(fingerprint).toMatch(/^[0-9a-f]{64}$/)
    expect(await crypto.ed25519.fingerprint(publicKey)).toBe(fingerprint)
  })
})
