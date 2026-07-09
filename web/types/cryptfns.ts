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

  /**
   * Present on migrated accounts. `"curve25519"` marks an Ed25519 identity key
   * whose file-key wrapping is done with the paired X25519 keys below.
   */
  keyType?: string

  /**
   * X25519 private key used to unwrap file keys on curve25519 accounts.
   */
  wrappingPrivate?: string | null

  /**
   * X25519 public key used to wrap file keys on curve25519 accounts.
   */
  wrappingPublic?: string | null
}

/**
 * AES key
 */
export type Key = Uint8Array
