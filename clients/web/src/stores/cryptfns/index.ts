import * as rsa from './rsa'
import * as aes from './aes'
import * as sha256 from './sha256'
import * as lscache from 'lscache'

export { rsa, aes, sha256 }

const ENCRYPTED_PRIVATE_KEY_LOCAL_STORAGE = 'encrypted-secret'

/**
 * Convert all sorts of arrays into a regular Buffer
 */
export function typedarrayToBuffer(
  array: Uint8Array | Uint16Array | Uint32Array | ArrayBuffer | Buffer
) {
  if (array instanceof Buffer) {
    return array
  }

  return ArrayBuffer.isView(array)
    ? // To avoid a copy, use the typed array's underlying ArrayBuffer to back
      // new Buffer, respecting the "view", i.e. byteOffset and byteLength
      Buffer.from(array.buffer, array.byteOffset, array.byteLength)
    : // Pass through all other types to `Buffer.from`
      Buffer.from(array)
}

/**
 * Get the encrypted private key from the localStorage
 */
export function getEncryptedPrivateKey(): string | null {
  return lscache.get(ENCRYPTED_PRIVATE_KEY_LOCAL_STORAGE)
}

/**
 * Lets us know if we should even attempt the decryption
 */
export function hasEncryptedPrivateKey(): boolean {
  return !!getEncryptedPrivateKey()
}

/**
 * Take the given private key, encrypt it with a pin and store it in localStorage
 */
export function encryptPrivateKeyAndStore(pk: string, pin: string) {
  const encrypted = aes.encryptString(pk, pin)

  lscache.set(ENCRYPTED_PRIVATE_KEY_LOCAL_STORAGE, encrypted)
}

/**
 * Remove the encrypted private key from storage
 */
export function clear() {
  if (hasEncryptedPrivateKey()) {
    lscache.remove(ENCRYPTED_PRIVATE_KEY_LOCAL_STORAGE)
  }
}

/**
 * Get the encrypted private key from storage and decrypt it
 */
export function getAndDecryptPrivateKey(pin: string) {
  const pk = getEncryptedPrivateKey()

  if (!pk) {
    throw new Error('No encrypted private key found')
  }

  return aes.decryptString(pk, pin)
}

/**
 * Create a timed nonce for authentication via private key
 */
export function createFingerprintNonce(fingerprint: string): string {
  const timestamp = parseInt(`${Date.now() / 1000}`)
  const flat = `${parseInt(`${timestamp / 60}`)}`

  return `${fingerprint}-${flat}`
}
