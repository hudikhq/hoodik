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

describe('Cipher chunk format test', () => {
  // CHUNK_FORMAT_TAG (8 bytes) followed by the nonce; see cryptfns::cipher.
  const CHUNK_FORMAT_TAG = new Uint8Array([0x48, 0x44, 0x4b, 0x43, 0x48, 0x4b, 0x00, 0x02])
  const NONCE_LENGTH: Record<string, number> = {
    ascon128a: 16,
    chacha20poly1305: 12,
    aegis128l: 16,
    aegis256: 32
  }
  const KEY_LENGTH: Record<string, number> = {
    ascon128a: 16,
    chacha20poly1305: 32,
    aegis128l: 16,
    aegis256: 32
  }
  const header = (cipher: string) => CHUNK_FORMAT_TAG.length + NONCE_LENGTH[cipher]

  it('UNIT: CIPHER: chunks round-trip for every cipher', async () => {
    for (const cipher of ALL_CIPHERS) {
      const key = await crypto.cipher.generateKey(cipher)
      for (let i = 0; i < 3; i++) {
        const data = new Uint8Array(64).fill(i + 1)
        const chunk = await crypto.cipher.encrypt(cipher, data, key, i, 3)
        expect(await crypto.cipher.decrypt(cipher, chunk, key, i, 3), cipher).toEqual(data)
      }
    }
  })

  it('UNIT: CIPHER: same key and index never share a keystream', async () => {
    for (const cipher of ALL_CIPHERS) {
      const key = await crypto.cipher.generateKey(cipher)
      const zeros = new Uint8Array(64)
      const ones = new Uint8Array(64).fill(0xff)

      const a = await crypto.cipher.encrypt(cipher, zeros, key, 0, 1)
      const b = await crypto.cipher.encrypt(cipher, ones, key, 0, 1)
      const h = header(cipher)

      expect(crypto.uint8.toHex(a.slice(0, h)), cipher).not.toBe(crypto.uint8.toHex(b.slice(0, h)))
      const xored = a.slice(h, h + 64).map((x, i) => x ^ b[h + i])
      expect(xored.every((x) => x === 0xff), cipher).toBe(false)

      const again = await crypto.cipher.encrypt(cipher, zeros, key, 0, 1)
      expect(crypto.uint8.toHex(again), cipher).not.toBe(crypto.uint8.toHex(a))
    }
  })

  it('UNIT: CIPHER: wrong index, wrong chunk count and tampering fail', async () => {
    const key = await crypto.cipher.generateKey('aegis128l')
    const data = new Uint8Array(16).fill(0x99)
    const chunk = await crypto.cipher.encrypt('aegis128l', data, key, 2, 4)

    await expect(crypto.cipher.decrypt('aegis128l', chunk, key, 3, 4)).rejects.toThrow()
    await expect(crypto.cipher.decrypt('aegis128l', chunk, key, 2, 3)).rejects.toThrow()

    const tampered = chunk.slice()
    tampered[tampered.length - 1] ^= 1
    await expect(crypto.cipher.decrypt('aegis128l', tampered, key, 2, 4)).rejects.toThrow()

    await expect(
      crypto.cipher.decrypt('aegis128l', chunk.slice(0, chunk.length - 1), key, 2, 4)
    ).rejects.toThrow()
  })

  it('UNIT: CIPHER: legacy fixed-nonce chunks still decrypt', async () => {
    for (const cipher of ALL_CIPHERS) {
      const key = await crypto.cipher.generateKey(cipher)

      for (let i = 0; i < 3; i++) {
        const data = new Uint8Array(48).fill(i + 1)
        const legacy = crypto.wasm.cipher_encrypt(cipher, key, data)

        assert(legacy, `legacy encrypt failed for "${cipher}"`)
        expect(await crypto.cipher.decrypt(cipher, legacy, key, i, 3), cipher).toEqual(data)
      }
    }
  })

  it('UNIT: CIPHER: legacy index-derived chunks still decrypt', async () => {
    for (const cipher of ALL_CIPHERS) {
      const key = await crypto.cipher.generateKey(cipher)

      for (const i of [0, 1, 5]) {
        // Legacy chunks derived their nonce by XOR-ing the little-endian
        // index into the key blob's embedded nonce.
        const legacyKey = key.slice()
        for (let b = 0; b < 8; b++) {
          legacyKey[KEY_LENGTH[cipher] + b] ^= Math.floor(i / 256 ** b) % 256
        }

        const data = new Uint8Array(48).fill(i + 1)
        const legacy = crypto.wasm.cipher_encrypt(cipher, legacyKey, data)

        assert(legacy, `legacy encrypt failed for "${cipher}"`)
        expect(await crypto.cipher.decrypt(cipher, legacy, key, i, 6), cipher).toEqual(data)
      }
    }
  })

  it('UNIT: CIPHER: a tagged chunk never falls back to the legacy decoder', async () => {
    for (const cipher of ALL_CIPHERS) {
      const key = await crypto.cipher.generateKey(cipher)
      const data = new Uint8Array(32).fill(0x2a)

      // A legacy ciphertext dressed with the format tag must fail outright
      // instead of being decoded by the legacy path.
      const legacy = crypto.wasm.cipher_encrypt(cipher, key, data)
      assert(legacy, `legacy encrypt failed for "${cipher}"`)

      const dressed = new Uint8Array(CHUNK_FORMAT_TAG.length + legacy.length)
      dressed.set(CHUNK_FORMAT_TAG)
      dressed.set(legacy, CHUNK_FORMAT_TAG.length)

      await expect(crypto.cipher.decrypt(cipher, dressed, key, 0, 1), cipher).rejects.toThrow()
    }
  })

  // Cross-client anchor for the tagged chunk layout: the same vector is
  // asserted in `cryptfns/src/cipher.rs` (chunk_golden_vector_decrypts) and
  // the Flutter client's `test/core/crypto/file_crypto_test.dart`. If any
  // implementation of the tag ‖ nonce ‖ ciphertext layout or the associated
  // data drifts, one of the three fails. Encryption cannot be goldened
  // because the nonce is random, so only decryption is pinned.
  it('UNIT: CIPHER: cross-client golden chunk vector decrypts', async () => {
    const key = crypto.uint8.fromHex('000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f')
    const chunk = crypto.uint8.fromHex(
      '48444b43484b0002a0a1a2a3a4a5a6a7a8a9aaabacadaeaf555f6e84c13c89dd00b3fe3a08937b3a8568eb1fbdc286e675a515fb'
    )

    expect(await crypto.cipher.decrypt('aegis128l', chunk, key, 1, 3)).toEqual(
      crypto.uint8.fromUtf8('hoodik chunk')
    )
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
