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
