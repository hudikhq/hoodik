import { AES_CTR } from '@openpgp/asmcrypto.js'
import { uint8 } from '.'

export type Key = {
  password: Uint8Array
  counter: Uint8Array
  blocksize?: number
}

/**
 * Convert a key into a string json
 */
export function keyToStringJson(key: Key): string {
  return JSON.stringify({
    password: uint8.toBase64(key.password),
    counter: uint8.toBase64(key.counter),
    blocksize: key.blocksize
  })
}

/**
 * Get a key from a string json
 */
export function keyFromStringJson(key: string): Key {
  const raw = JSON.parse(key)

  return {
    password: uint8.fromBase64(raw.password),
    counter: uint8.fromBase64(raw.counter),
    blocksize: raw.blocksize
  }
}

/**
 * When creating a new file generate a random key for it
 * that will be used for actual data encryption
 */
export function generateKey(blocksize: number = 128): Key {
  const password = generateRandomUint8Array(32)
  const counter = generateRandomUint8Array(16)

  return {
    password,
    counter,
    blocksize
  }
}

/**
 * Turn a normal string into a key
 */
export function keyFromString(key: string): Key {
  const encoder = new TextEncoder()

  while (key.length < 32) {
    key += '0'
  }

  const password = encoder.encode(key.slice(0, 32))
  const counter = encoder.encode(key.slice(0, 16))

  return {
    password,
    counter
  }
}

/**
 * Randomly generate a Uint8Array of the selected length
 */
export function generateRandomUint8Array(length: number): Uint8Array {
  const key = new Uint8Array(length)

  for (let i = 0; i < key.length; i++) {
    key.set([Math.floor(Math.random() * 256)], i)
  }

  return key
}

/**
 * Create an aes operator for encrypting and decrypting
 */
function getOperator(key: Key) {
  return new AES_CTR(key.password, key.counter)
}

/**
 * Encrypt raw data with the selected key
 */
export function encrypt(data: Uint8Array, key: Key): Uint8Array {
  const blocksize = key.blocksize || 128

  for (let i = 0; i < data.length; i += blocksize) {
    const encrypted = getOperator(key).encrypt(data.slice(i, i + blocksize))

    data.set(encrypted, i)
  }

  return data
}

/**
 * Encrypt raw data with the selected key
 */
export function decrypt(data: Uint8Array, key: Key, blocksize?: number): Uint8Array {
  blocksize = blocksize || key.blocksize || 128

  for (let i = 0; i < data.length; i += blocksize) {
    const decrypted = getOperator(key).decrypt(data.slice(i, i + blocksize))

    data.set(decrypted, i)
  }

  return data
}

/**
 * Concat multiple uint8Arrays together
 */
export function concatUint8Array(...arrays: Uint8Array[]): Uint8Array {
  let totalLength = 0
  for (let i = 0; i < arrays.length; i++) {
    totalLength += arrays[i].length
  }

  const result = new Uint8Array(totalLength)
  let offset = 0
  for (let i = 0; i < arrays.length; i++) {
    result.set(arrays[i], offset)
    offset += arrays[i].length
  }

  return result
}

/**
 * Encrypt a string and return a string
 */
export function encryptString(secret: string, key: string | Key): string {
  key = typeof key === 'string' ? keyFromString(key) : key

  const secretBuffer = uint8.fromUtf8(secret)
  const result = encrypt(secretBuffer, key)
  return uint8.toHex(result)
}

/**
 * Decrypt a string and return a string
 */
export function decryptString(secret: string, key: string | Key): string {
  key = typeof key === 'string' ? keyFromString(key) : key

  const secretBuffer = uint8.fromHex(secret)
  const result = decrypt(secretBuffer, key)
  return uint8.toUtf8(result)
}
