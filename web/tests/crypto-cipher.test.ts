import { afterEach, describe, it, expect, assert } from 'vitest'
import * as crypto from '../services/cryptfns'

describe('Cipher test', () => {
  afterEach(() => {
    crypto.cipher.setDefaultCipher('aegis128l')
  })

  it('UNIT: CIPHER: can encrypt and decrypt string with AEGIS-256', async () => {
    const secret = 'little secret'
    const key = await crypto.cipher.generateKey('aegis256')
    const encrypted = await crypto.cipher.encryptString('aegis256', secret, key)

    assert(encrypted !== secret, 'Encrypted value is the same as provided value')

    const decrypted = await crypto.cipher.decryptString('aegis256', encrypted, key)

    expect(decrypted).toBe(secret)
  })

  it('UNIT: CIPHER: can encrypt and decrypt multi-bytes characters with AEGIS-256', async () => {
    const secret = 'あいうえお'
    const key = await crypto.cipher.generateKey('aegis256')
    const encrypted = await crypto.cipher.encryptString('aegis256', secret, key)

    assert(encrypted !== secret, 'Encrypted value is the same as provided value')

    const decrypted = await crypto.cipher.decryptString('aegis256', encrypted, key)

    expect(decrypted).toBe(secret)
  })

  it('UNIT: CIPHER: default cipher starts as aegis128l', () => {
    expect(crypto.cipher.defaultCipher()).toBe('aegis128l')
  })

  it('UNIT: CIPHER: setDefaultCipher changes the cipher generateKey uses', async () => {
    const before = await crypto.cipher.generateKey()
    expect(before.length).toBe(32)

    crypto.cipher.setDefaultCipher('aegis256')
    expect(crypto.cipher.defaultCipher()).toBe('aegis256')

    const after = await crypto.cipher.generateKey()
    expect(after.length).toBe(64)
  })
})

const ALL_CIPHERS = ['ascon128a', 'chacha20poly1305', 'aegis128l', 'aegis256']

describe('Cipher chunk nonce test', () => {
  it('UNIT: CIPHER: identical chunks at different indices produce different ciphertext', async () => {
    for (const cipher of ALL_CIPHERS) {
      const key = await crypto.cipher.generateKey(cipher)
      const data = new Uint8Array(64).fill(0x42)

      const chunk0 = await crypto.cipher.encrypt(cipher, data, key, 0)
      const chunk1 = await crypto.cipher.encrypt(cipher, data, key, 1)

      expect(crypto.uint8.toHex(chunk0), cipher).not.toBe(crypto.uint8.toHex(chunk1))
      expect(await crypto.cipher.decrypt(cipher, chunk0, key, 0)).toEqual(data)
      expect(await crypto.cipher.decrypt(cipher, chunk1, key, 1)).toEqual(data)
    }
  })

  it('UNIT: CIPHER: chunk 0 is byte-identical to the legacy single-chunk output', async () => {
    for (const cipher of ALL_CIPHERS) {
      const key = await crypto.cipher.generateKey(cipher)
      const data = new Uint8Array(32).fill(0x07)

      const chunk0 = await crypto.cipher.encrypt(cipher, data, key, 0)
      const legacy = crypto.wasm.cipher_encrypt(cipher, key, data)

      expect(chunk0, cipher).toEqual(legacy)
    }
  })

  it('UNIT: CIPHER: legacy fixed-nonce multi-chunk files still decrypt', async () => {
    for (const cipher of ALL_CIPHERS) {
      const key = await crypto.cipher.generateKey(cipher)

      for (let i = 0; i < 3; i++) {
        const data = new Uint8Array(48).fill(i + 1)
        const legacy = crypto.wasm.cipher_encrypt(cipher, key, data)

        assert(legacy, `legacy encrypt failed for "${cipher}"`)
        expect(await crypto.cipher.decrypt(cipher, legacy, key, i), cipher).toEqual(data)
      }
    }
  })

  it('UNIT: CIPHER: a chunk decrypted at the wrong index fails authentication', async () => {
    const key = await crypto.cipher.generateKey('aegis128l')
    const data = new Uint8Array(16).fill(0x99)
    const chunk = await crypto.cipher.encrypt('aegis128l', data, key, 2)

    await expect(crypto.cipher.decrypt('aegis128l', chunk, key, 3)).rejects.toThrow()
  })
})

describe('Cipher metadata nonce test', () => {
  it('UNIT: CIPHER: metadata strings never reuse a nonce under the same key', async () => {
    for (const cipher of ALL_CIPHERS) {
      const key = await crypto.cipher.generateKey(cipher)

      const first = await crypto.cipher.encryptString(cipher, 'secret.txt', key)
      const second = await crypto.cipher.encryptString(cipher, 'secret.txt', key)

      expect(first, cipher).not.toBe(second)
      expect(await crypto.cipher.decryptString(cipher, first, key)).toBe('secret.txt')
      expect(await crypto.cipher.decryptString(cipher, second, key)).toBe('secret.txt')
    }
  })

  it('UNIT: CIPHER: legacy embedded-nonce metadata still decrypts', async () => {
    for (const cipher of ALL_CIPHERS) {
      const key = await crypto.cipher.generateKey(cipher)
      const legacy = crypto.wasm.cipher_encrypt(cipher, key, crypto.uint8.fromUtf8('old name.txt'))

      assert(legacy, `legacy encrypt failed for "${cipher}"`)
      expect(await crypto.cipher.decryptString(cipher, crypto.uint8.toHex(legacy), key), cipher).toBe(
        'old name.txt'
      )
    }
  })

  // Cross-client anchor: the same vector is asserted in
  // `cryptfns/src/cipher.rs` (string_golden_vector_decrypts) and the Flutter
  // client's `test/core/crypto/file_crypto_test.dart`. If any client's
  // nonce-prepend format drifts, one of the three fails.
  it('UNIT: CIPHER: cross-client golden vector decrypts', async () => {
    const key = crypto.uint8.fromHex('000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f')
    const blob = 'a0a1a2a3a4a5a6a7a8a9aaabacadaeafbbe8a3087cc12efc536324b18fb194d014ab82478e8e43951d2d'

    expect(await crypto.cipher.decryptString('ascon128a', blob, key)).toBe('hoodik.txt')
  })
})
