import { describe, it, expect, assert } from 'vitest'
import * as crypto from '../../../src/stores/cryptfns'

describe('Crypto test', () => {
  it('UNIT: AES: can encrypt and decrypt string with provided pin using AES', async () => {
    const secret = 'little secret'
    const pin = '123456'
    const encrypted = crypto.aes.encryptString(secret, pin)

    assert(encrypted !== secret, 'Encrypted value is the same as provided value')

    const decrypted = crypto.aes.decryptString(encrypted, pin)

    expect(decrypted).toBe(secret)
  })

  it('UNIT: AES: FILE: Try encrypting multiple chunks and then decrypt them after the result was concated', () => {
    const singleSize = 1024 * 1024
    let d1 = ''
    for (let i = 0; i < singleSize; i++) {
      d1 += '1'
    }
    let d2 = ''
    for (let i = 0; i < singleSize; i++) {
      d2 += '2'
    }
    let d3 = ''
    for (let i = 0; i < singleSize + 3213; i++) {
      d3 += '3'
    }

    const all = `${d1}${d2}${d3}`

    const key = crypto.aes.generateKey(singleSize)

    const encoder = new TextEncoder()
    const decoder = new TextDecoder()

    const data1 = encoder.encode(d1)
    const data2 = encoder.encode(d2)
    const data3 = encoder.encode(d3)
    const concatPlain = encoder.encode(all)

    const encrypted1 = crypto.aes.encrypt(data1, key)
    const encrypted2 = crypto.aes.encrypt(data2, key)
    const encrypted3 = crypto.aes.encrypt(data3, key)

    const concatEncrypted = crypto.aes.concatUint8Array(encrypted1, encrypted2, encrypted3)
    const encryptedAll = crypto.aes.encrypt(concatPlain, key)

    // console.log('concatEncrypted', concatEncrypted)
    // console.log('encryptedAll', encryptedAll)

    const decryptedConcatEncrypted = crypto.aes.decrypt(concatEncrypted, key)
    const decryptedAll = crypto.aes.decrypt(encryptedAll, key)

    // console.log('decryptedConcatEncrypted', decryptedConcatEncrypted)
    // console.log('decryptedAll', decryptedAll)

    const decryptedStringConcat = decoder.decode(decryptedConcatEncrypted)
    const decryptedStringAll = decoder.decode(decryptedAll)

    expect(data1.length).toBe(encrypted1.length)
    expect(data2.length).toBe(encrypted2.length)
    expect(data3.length).toBe(encrypted3.length)
    expect(all.length).toBe(encryptedAll.length)
    expect(decryptedStringConcat).toBe(all)
    expect(decryptedStringAll.toString()).toBe(all)
  })
})
