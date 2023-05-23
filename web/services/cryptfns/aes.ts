import type { Key } from 'types'
import { uint8 } from '.'
import { init, aes_decrypt, aes_encrypt, aes_generate_key } from './wasm'

/**
 * Convert a key into a string json
 */
export function keyToStringJson(key: Key): string {
  return uint8.toHex(key)
}

/**
 * Get a key from a string json
 */
export function keyFromStringJson(key: string): Key {
  return uint8.fromHex(key)
}

/**
 * Take a regular string and convert it into a key
 */
export function keyFromSimpleString(input: string): Key {
  const targetLength = 32
  const lengthDifference = targetLength - input.length

  if (lengthDifference > 0) {
    input = input.padEnd(targetLength, '0')
  } else if (lengthDifference < 0) {
    input = input.substring(0, targetLength)
  }

  return uint8.fromUtf8(input)
}

/**
 * When creating a new file generate a random key for it
 * that will be used for actual data encryption
 */
export async function generateKey(): Promise<Key> {
  await init()
  const key = aes_generate_key()

  if (!key) {
    throw new Error('Failed to generate key')
  }

  return key
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
 * Encrypt raw data with the selected key
 */
export async function encrypt(data: Uint8Array, key: Key): Promise<Uint8Array> {
  await init()
  const ciphertext = aes_encrypt(key, data)

  if (!ciphertext) {
    throw new Error('Failed to encrypt data')
  }

  return ciphertext
}

/**
 * Encrypt raw data with the selected key
 */
export async function decrypt(ciphertext: Uint8Array, key: Key): Promise<Uint8Array> {
  await init()
  const plaintext = aes_decrypt(key, ciphertext)

  if (!plaintext) {
    throw new Error('Failed to decrypt ciphertext')
  }

  return plaintext
}

/**
 * Encrypt a string and return a string
 */
export async function encryptString(secret: string, key: string | Key): Promise<string> {
  key = typeof key === 'string' ? keyFromSimpleString(key) : key

  const plaintext = uint8.fromUtf8(secret)
  const result = await encrypt(plaintext, key)
  return uint8.toHex(result)
}

/**
 * Decrypt a string and return a string
 */
export async function decryptString(secret: string, key: string | Key): Promise<string> {
  key = typeof key === 'string' ? keyFromSimpleString(key) : key

  const ciphertext = uint8.fromHex(secret)
  const result = await decrypt(ciphertext, key)
  return uint8.toUtf8(result)
}

/**
 * Take an encrypted string that is supposed to be hex and decrypt it with given aes key
 */
export async function decryptFromHex(hex: string, key: Key): Promise<string> {
  return uint8.toUtf8(await decrypt(uint8.fromHex(hex), key))
}

/**
 * Take a string or Uint8Array, encrypt it with given aes key and return it as hex
 */
export async function encryptToHex(data: string | Uint8Array, key: Key): Promise<string> {
  if (typeof data === 'string') {
    data = uint8.fromUtf8(data)
  }

  return uint8.toHex(await encrypt(data, key))
}
