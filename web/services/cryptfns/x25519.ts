import {
  init,
  x25519_generate_private,
  x25519_public_from_private,
  x25519_wrap,
  x25519_unwrap
} from './wasm'

/**
 * Generate an X25519 private key as PKCS#8 PEM.
 */
export async function generatePrivateKey(): Promise<string> {
  await init()
  const privateKey = x25519_generate_private()

  if (!privateKey) {
    throw new Error('x25519_generate_private failed')
  }

  return privateKey
}

/**
 * Derive the SPKI PEM public key from a PKCS#8 PEM private key.
 */
export async function publicFromPrivate(privateKey: string): Promise<string> {
  await init()
  const publicKey = x25519_public_from_private(privateKey)

  if (!publicKey) {
    throw new Error('x25519_public_from_private failed')
  }

  return publicKey
}

/**
 * Wrap a file key for the recipient; returns a base64 ECIES blob.
 */
export async function wrap(fileKey: Uint8Array, recipientPublicKey: string): Promise<string> {
  await init()
  const blob = x25519_wrap(fileKey, recipientPublicKey)

  if (!blob) {
    throw new Error('x25519_wrap failed')
  }

  return blob
}

/**
 * Unwrap a base64 ECIES blob with the recipient's private key.
 */
export async function unwrap(blob: string, privateKey: string): Promise<Uint8Array> {
  await init()
  const fileKey = x25519_unwrap(blob, privateKey)

  if (!fileKey) {
    throw new Error('x25519_unwrap failed')
  }

  return fileKey
}
