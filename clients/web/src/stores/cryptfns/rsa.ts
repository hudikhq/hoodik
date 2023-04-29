import type { KeyPair } from '@/types'
import { aes } from '.'
import {
  rsa_generate_private,
  rsa_public_from_private,
  rsa_decrypt,
  rsa_encrypt,
  rsa_fingerprint_public,
  rsa_fingerprint_private,
  rsa_sign,
  rsa_verify,
  rsa_public_key_size,
  rsa_private_key_size
} from './wasm'

/**
 * Decrypt a private key
 * @throws
 */
export async function decryptPrivateKey(encrypted: string, passphrase: string): Promise<string> {
  return aes.decryptString(encrypted, passphrase)
}

/**
 * Protect private key with passphrase
 * @throws
 */
export async function protectPrivateKey(unencrypted: string, passphrase: string): Promise<string> {
  return aes.encryptString(unencrypted, passphrase)
}

/**
 * Convert input to raw
 */
export async function inputToKeyPair(input: string): Promise<KeyPair> {
  const publicKey = rsa_public_from_private(input)

  if (!publicKey) {
    throw new Error('Invalid private key')
  }

  const fingerprint = rsa_fingerprint_public(publicKey)

  if (!fingerprint) {
    throw new Error('Invalid private key')
  }

  const keySize = rsa_private_key_size(input)

  if (!keySize) {
    throw new Error('Invalid private key')
  }

  return {
    publicKey,
    input,
    fingerprint,
    keySize
  }
}

/**
 * Generate KeyPair from public key, this can only be used to verify signatures
 */
export async function publicToKeyPair(publicKey: string): Promise<KeyPair> {
  console.log(publicKey)
  const fingerprint = rsa_fingerprint_public(publicKey)

  if (!fingerprint) {
    throw new Error('Invalid public key')
  }

  const keySize = rsa_public_key_size(publicKey)

  if (!keySize) {
    throw new Error('Invalid public key')
  }

  return { publicKey, input: null, fingerprint, keySize }
}

/**
 * Generate key id from string
 *
 * @throws
 */
export async function getFingerprint(input: string): Promise<string> {
  let fingerprint = rsa_fingerprint_public(input)

  if (!fingerprint) {
    fingerprint = rsa_fingerprint_private(input)
  }

  if (!fingerprint) {
    throw new Error('Invalid key')
  }

  return fingerprint
}

/**
 * Generate a random input in a format of KeyPair
 */
export async function generateKeyPair(): Promise<KeyPair> {
  const privateKey = rsa_generate_private()

  if (!privateKey) {
    throw new Error('Could not generate private key')
  }

  return inputToKeyPair(privateKey)
}

/**
 * Sign the given message with current secret key and return an object with signature and publicKey
 */
export async function sign(kp: KeyPair, message: string): Promise<string> {
  const { input } = kp

  if (!input) {
    throw new Error('No privateKey, cannot sign message')
  }

  const signature = rsa_sign(message, input)

  if (!signature) {
    throw new Error('Could not sign message')
  }

  return signature
}

/**
 * Verify the message with the given public key or the stored one
 */
export async function verify(
  signature: string,
  message: string,
  publicKey: string
): Promise<boolean> {
  const { publicKey: pk } = await publicToKeyPair(publicKey)

  if (!pk) {
    throw new Error('No publicKey, cannot verify message')
  }

  return rsa_verify(message, signature, pk)
}

/**
 * Encrypt a message with given public key
 */
export async function encryptMessage(message: string, publicKey: string): Promise<string> {
  const fingerprint = await getFingerprint(publicKey)

  if (!fingerprint) {
    throw new Error('Invalid public key')
  }

  const encrypted = rsa_encrypt(message, publicKey)

  if (!encrypted) {
    throw new Error('Could not encrypt message')
  }

  return encrypted
}

/**
 * Decrypt a message with stored private key
 */
export async function decryptMessage(kp: KeyPair, message: string): Promise<string> {
  const { input } = kp

  if (!input) {
    throw new Error('Invalid private key')
  }

  const decrypted = rsa_decrypt(message, input)

  if (!decrypted) {
    throw new Error('Could not decrypt message')
  }

  return decrypted
}
