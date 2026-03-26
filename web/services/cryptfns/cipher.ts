import type { Key } from 'types'
import { uint8 } from '.'
import { init, cipher_generate_key, cipher_encrypt, cipher_decrypt } from './wasm'

/**
 * Default cipher identifier — matches the Rust `cryptfns::cipher::DEFAULT` constant.
 * Change this single value to experiment with a different cipher for new file uploads.
 * Existing files always use whatever cipher is stored in their `cipher` DB field.
 */
export const DEFAULT_CIPHER = 'aegis128l'

/**
 * Generate a random key suitable for the given cipher.
 *
 * - `"ascon128a"`:        returns 32 bytes (16-byte key + 16-byte nonce)
 * - `"chacha20poly1305"`: returns 44 bytes (32-byte key + 12-byte nonce)
 * - `"aegis128l"`:        returns 32 bytes (16-byte key + 16-byte nonce)
 */
export async function generateKey(cipher = DEFAULT_CIPHER): Promise<Key> {
  await init()
  const key = cipher_generate_key(cipher)

  if (!key) {
    throw new Error(`cipher_generate_key failed for "${cipher}"`)
  }

  return key
}

/**
 * Encrypt raw bytes with the given cipher and key.
 */
export async function encrypt(cipher: string, data: Uint8Array, key: Key): Promise<Uint8Array> {
  await init()
  const ciphertext = cipher_encrypt(cipher, key, data)

  if (!ciphertext) {
    throw new Error(`cipher_encrypt failed for "${cipher}"`)
  }

  return ciphertext
}

/**
 * Decrypt raw bytes with the given cipher and key.
 */
export async function decrypt(cipher: string, ciphertext: Uint8Array, key: Key): Promise<Uint8Array> {
  await init()
  const plaintext = cipher_decrypt(cipher, key, ciphertext)

  if (!plaintext) {
    throw new Error(`cipher_decrypt failed for "${cipher}"`)
  }

  return plaintext
}

/**
 * Encrypt a UTF-8 string and return the result as a hex string.
 * Mirrors the encoding convention used by `aes.encryptString`.
 */
export async function encryptString(cipher: string, secret: string, key: Key): Promise<string> {
  const plaintext = uint8.fromUtf8(secret)
  const result = await encrypt(cipher, plaintext, key)
  return uint8.toHex(result)
}

/**
 * Decrypt a hex-encoded ciphertext string and return the UTF-8 plaintext.
 * Mirrors the encoding convention used by `aes.decryptString`.
 */
export async function decryptString(cipher: string, hex: string, key: Key): Promise<string> {
  const ciphertext = uint8.fromHex(hex)
  const result = await decrypt(cipher, ciphertext, key)
  return uint8.toUtf8(result)
}
