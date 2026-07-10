import {
  init,
  wrapping_generate_private,
  wrapping_public_from_private,
  wrapping_wrap,
  wrapping_unwrap
} from './wasm'

/**
 * Generate a hybrid X25519 + ML-KEM-768 wrapping private key, PEM-encoded.
 */
export async function generatePrivateKey(): Promise<string> {
  await init()
  const privateKey = wrapping_generate_private()

  if (!privateKey) {
    throw new Error('wrapping_generate_private failed')
  }

  return privateKey
}

/**
 * Derive the wrapping public key (PEM) from a wrapping private key.
 */
export async function publicFromPrivate(privateKey: string): Promise<string> {
  await init()
  const publicKey = wrapping_public_from_private(privateKey)

  if (!publicKey) {
    throw new Error('wrapping_public_from_private failed')
  }

  return publicKey
}

/**
 * Wrap a file key for the recipient; returns a base64 hybrid-wrap blob.
 */
export async function wrap(fileKey: Uint8Array, recipientPublicKey: string): Promise<string> {
  await init()
  const blob = wrapping_wrap(fileKey, recipientPublicKey)

  if (!blob) {
    throw new Error('wrapping_wrap failed')
  }

  return blob
}

/**
 * Unwrap a base64 hybrid-wrap blob with the recipient's private key.
 */
export async function unwrap(blob: string, privateKey: string): Promise<Uint8Array> {
  await init()
  const fileKey = wrapping_unwrap(blob, privateKey)

  if (!fileKey) {
    throw new Error('wrapping_unwrap failed')
  }

  return fileKey
}
