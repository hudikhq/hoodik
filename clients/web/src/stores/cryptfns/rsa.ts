import RSA from 'node-rsa'
import { aes, sha256 } from '.'
import * as JSE from 'jsencrypt'
const JSEncrypt = JSE.default
import { Buffer } from 'buffer'

const RSA_BYTES = 2048
const ENVIRONMENT = 'browser'
const PRIVATE_KEY_FORMAT: RSA.FormatPem = 'pkcs1'
const PUBLIC_KEY_FORMAT: RSA.FormatPem = 'pkcs1-public-pem'
const SIGNING_SCHEME = 'pss-sha256'
const RSA_OPTIONS: RSA.Options = {
  environment: ENVIRONMENT,
  signingScheme: SIGNING_SCHEME
}
// const ENCRYPTION_SCHEME_PKCS1: RSA.AdvancedEncryptionSchemePKCS1 = {
//   scheme: 'pkcs1',
//   padding: 1
// }

const JSE_ENCRYPT_OPTIONS = { default_key_size: RSA_BYTES.toString() }

export interface Raw extends RSA {
  input?: string
}

export type Encoding = RSA.Encoding
export type Data = RSA.Data

export interface EncryptionData {
  message: Data
  encoding?: Encoding
}

export interface KeyPair {
  /**
   * Private RSA key string
   */
  input: string | null

  /**
   * The RSA key
   */
  key: Raw | null

  /**
   * Public RSA key string
   */
  publicKey: string | null

  /**
   * Fingerprint of the public key
   */
  fingerprint: string | null
}

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
  const key: Raw = new RSA({ b: RSA_BYTES }).importKey(input, PRIVATE_KEY_FORMAT)
  key.setOptions(RSA_OPTIONS)

  key.input = input

  const publicKey = key.exportKey(PUBLIC_KEY_FORMAT)
  const fingerprint = await getFingerprint(input)

  return {
    key,
    publicKey,
    input,
    fingerprint
  }
}

/**
 * Generate KeyPair from public key, this can only be used to verify signatures
 */
export async function publicToKeyPair(publicKey: string): Promise<KeyPair> {
  const key = new RSA(publicKey, PUBLIC_KEY_FORMAT, RSA_OPTIONS)

  return { key, publicKey, input: null, fingerprint: await getFingerprint(publicKey) }
}

/**
 * Generate key id from string
 *
 * @throws
 */
export async function getFingerprint(input: string): Promise<string> {
  const operator = new JSEncrypt(JSE_ENCRYPT_OPTIONS)
  operator.setKey(input)
  // @ts-ignore
  const n = operator.getKey().n.toString(16)

  return sha256.digest(n)
}

/**
 * Generate a random input in a format of KeyPair
 */
export async function generateKeyPair(): Promise<KeyPair> {
  return inputToKeyPair(new RSA({ b: RSA_BYTES }).generateKeyPair().exportKey(PRIVATE_KEY_FORMAT))
}

/**
 * Generate a KeyPair from input
 */
export async function keypairFromRaw(internal: KeyPair): Promise<KeyPair> {
  const { key, publicKey } = internal

  let fingerprint = null

  if (publicKey) {
    fingerprint = await getFingerprint(publicKey)
  }

  return {
    input: key?.input || null,
    key,
    publicKey,
    fingerprint
  }
}

/**
 * Sign the given message with current secret key and return an object with signature and publicKey
 */
export async function sign(kp: KeyPair, message: string): Promise<string> {
  const { key } = kp

  if (!key || !key.isPrivate()) {
    throw new Error('No privateKey, cannot sign message')
  }

  return key.sign(message, 'base64')
}

/**
 * Verify the message with the given public key or the stored one
 */
export async function verify(
  signature: string,
  message: string,
  publicKey: string
): Promise<boolean> {
  const { key } = await publicToKeyPair(publicKey)

  if (!key) {
    throw new Error('No publicKey, cannot verify message')
  }

  return key.verify(message, Buffer.from(signature, 'base64'))
}

/**
 * Encrypt a message with given public key
 */
// export async function encryptMessage(message: string, publicKey: string): Promise<string> {
//   const { key } = await publicToKeyPair(publicKey as string)

//   if (!key) {
//     throw new Error('No publicKey, cannot encrypt message')
//   }

//   if (!key.isPublic()) {
//     throw new Error('Key is not public, cannot encrypt message')
//   }

//   key.setOptions({
//     encryptionScheme: ENCRYPTION_SCHEME_PKCS1
//   })

//   return key.encrypt(message, 'base64')
// }

/**
 * Encrypt a message with given public key
 */
export async function encryptMessage(message: string, publicKey: string): Promise<string> {
  const operator = new JSEncrypt(JSE_ENCRYPT_OPTIONS)
  operator.setPublicKey(publicKey as string)

  return operator.encrypt(message) as string
}

/**
 * Decrypt a message with stored private key
 */
export async function decryptMessage(kp: KeyPair, message: string): Promise<string> {
  const operator = new JSEncrypt(JSE_ENCRYPT_OPTIONS)
  operator.setPrivateKey(kp.input as string)

  return operator.decrypt(message) as string
}
