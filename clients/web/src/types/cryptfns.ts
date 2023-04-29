export interface KeyPair {
  /**
   * Private RSA key string
   */
  input: string | null

  /**
   * Public RSA key string
   */
  publicKey: string | null

  /**
   * Fingerprint of the public key
   */
  fingerprint: string | null

  /**
   * Size of the key in bits
   */
  keySize: number
}

/**
 * AES key
 */
export type Key = Uint8Array
