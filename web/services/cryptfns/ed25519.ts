import {
  init,
  ed25519_generate_private,
  ed25519_public_from_private,
  ed25519_sign,
  ed25519_verify,
  ed25519_sign_bytes,
  ed25519_verify_bytes,
  spki_fingerprint
} from './wasm'

/**
 * Generate an Ed25519 private key as PKCS#8 PEM.
 */
export async function generatePrivateKey(): Promise<string> {
  await init()
  const privateKey = ed25519_generate_private()

  if (!privateKey) {
    throw new Error('ed25519_generate_private failed')
  }

  return privateKey
}

/**
 * Derive the SPKI PEM public key from a PKCS#8 PEM private key.
 */
export async function publicFromPrivate(privateKey: string): Promise<string> {
  await init()
  const publicKey = ed25519_public_from_private(privateKey)

  if (!publicKey) {
    throw new Error('ed25519_public_from_private failed')
  }

  return publicKey
}

/**
 * Sign a message; returns the signature as base64.
 */
export async function sign(message: string, privateKey: string): Promise<string> {
  await init()
  const signature = ed25519_sign(message, privateKey)

  if (!signature) {
    throw new Error('ed25519_sign failed')
  }

  return signature
}

/**
 * Sign raw bytes; returns the signature as base64.
 */
export async function signBytes(message: Uint8Array, privateKey: string): Promise<string> {
  await init()
  const signature = ed25519_sign_bytes(message, privateKey)

  if (!signature) {
    throw new Error('ed25519_sign_bytes failed')
  }

  return signature
}

/**
 * Verify a base64 signature over a message.
 */
export async function verify(
  message: string,
  signature: string,
  publicKey: string
): Promise<boolean> {
  await init()
  return ed25519_verify(message, signature, publicKey)
}

/**
 * Verify a base64 signature over raw bytes.
 */
export async function verifyBytes(
  message: Uint8Array,
  signature: string,
  publicKey: string
): Promise<boolean> {
  await init()
  return ed25519_verify_bytes(message, signature, publicKey)
}

/**
 * Fingerprint of an SPKI PEM public key (64 hex characters).
 */
export async function fingerprint(publicKey: string): Promise<string> {
  await init()
  const result = spki_fingerprint(publicKey)

  if (!result) {
    throw new Error('spki_fingerprint failed')
  }

  return result
}
