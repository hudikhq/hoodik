import type { Key } from 'types'
import {
  init,
  cipher_generate_key,
  cipher_encrypt_string,
  cipher_decrypt_string,
  cipher_encrypt_chunk,
  cipher_decrypt_chunk
} from './wasm'

let currentDefaultCipher = 'aegis128l'

/**
 * Cipher identifier used for new file uploads. The value comes from the
 * server's `GET /api/capabilities` advertisement; until that response
 * arrives it matches the Rust `cryptfns::cipher::DEFAULT` constant.
 * Existing files always use whatever cipher is stored in their `cipher` DB field.
 */
export function defaultCipher(): string {
  return currentDefaultCipher
}

export function setDefaultCipher(cipher: string): void {
  currentDefaultCipher = cipher
}

/**
 * Generate a random key suitable for the given cipher.
 *
 * - `"ascon128a"`:        returns 32 bytes (16-byte key + 16-byte nonce)
 * - `"chacha20poly1305"`: returns 44 bytes (32-byte key + 12-byte nonce)
 * - `"aegis128l"`:        returns 32 bytes (16-byte key + 16-byte nonce)
 * - `"aegis256"`:         returns 64 bytes (32-byte key + 32-byte nonce)
 */
export async function generateKey(cipher = defaultCipher()): Promise<Key> {
  await init()
  const key = cipher_generate_key(cipher)

  if (!key) {
    throw new Error(`cipher_generate_key failed for "${cipher}"`)
  }

  return key
}

/**
 * Encrypt one chunk of a file with the given cipher and key.
 *
 * The chunk index is folded into the key's embedded nonce so no two
 * chunks of a file share one — encrypting every chunk with the blob
 * as-is reuses the nonce and voids the AEAD guarantees. Index 0 leaves
 * the nonce untouched, so single-chunk output is unchanged.
 */
export async function encrypt(
  cipher: string,
  data: Uint8Array,
  key: Key,
  chunkIndex: number
): Promise<Uint8Array> {
  await init()
  const ciphertext = cipher_encrypt_chunk(cipher, key, chunkIndex, data)

  if (!ciphertext) {
    throw new Error(`cipher_encrypt_chunk failed for "${cipher}"`)
  }

  return ciphertext
}

/**
 * Decrypt one chunk of a file.
 *
 * Tries the per-chunk nonce first, then falls back to the legacy
 * fixed-nonce scheme so files uploaded before per-chunk nonces still
 * decrypt — the AEAD tag rejects whichever branch is wrong.
 */
export async function decrypt(
  cipher: string,
  ciphertext: Uint8Array,
  key: Key,
  chunkIndex: number
): Promise<Uint8Array> {
  await init()
  const plaintext = cipher_decrypt_chunk(cipher, key, chunkIndex, ciphertext)

  if (!plaintext) {
    throw new Error(`cipher_decrypt_chunk failed for "${cipher}"`)
  }

  return plaintext
}

/**
 * Encrypt a UTF-8 string and return the result as a hex string.
 * Mirrors the encoding convention used by `aes.encryptString`.
 *
 * Metadata strings (file names, thumbnails) share the file key with the
 * content chunks and with each other — under the key's embedded nonce
 * they would all reuse the same (key, nonce) pair. Each string therefore
 * gets a fresh random nonce, prepended to the ciphertext. The format
 * lives in `cryptfns::cipher::Cipher::encrypt_string` so every client
 * produces identical output.
 */
export async function encryptString(cipher: string, secret: string, key: Key): Promise<string> {
  await init()
  const encrypted = cipher_encrypt_string(cipher, secret, key)

  if (typeof encrypted !== 'string') {
    throw new Error(`cipher_encrypt_string failed for "${cipher}"`)
  }

  return encrypted
}

/**
 * Decrypt a hex-encoded ciphertext string and return the UTF-8 plaintext.
 * Mirrors the encoding convention used by `aes.decryptString`.
 *
 * Tries the random-nonce format first (nonce prepended by
 * `encryptString`), then falls back to the legacy layout that used the
 * key's embedded nonce — the AEAD tag rejects whichever branch is wrong.
 */
export async function decryptString(cipher: string, hex: string, key: Key): Promise<string> {
  await init()
  const decrypted = cipher_decrypt_string(cipher, hex, key)

  if (typeof decrypted !== 'string') {
    throw new Error(`cipher_decrypt_string failed for "${cipher}"`)
  }

  return decrypted
}
