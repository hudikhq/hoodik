export interface CreateLink {
  /**
   * Id of the file that will be shared.
   */
  file_id: string

  /**
   * RSA signature where the message is file_id.
   */
  signature: string

  /**
   * Encrypted name of the file with file key
   */
  encrypted_name: string

  /**
   * Link key encrypted:
   *  - convert key to HEX
   *  - encrypt with users RSA public key
   */
  encrypted_link_key: string

  /**
   * File thumbnail encrypted with link key
   */
  encrypted_thumbnail?: string

  /**
   * AES file key converted to hex, encrypted with link key and then again converted to hex
   */
  encrypted_file_key: string

  /**
   * Expiration date of the link
   */
  expires_at?: string
}

export interface AppLink extends EncryptedAppLink {
  name: string
  thumbnail?: string
  link_key: Uint8Array
  link_key_hex: string

  /**
   * File content key, unwrapped from `encrypted_file_key` with the link key.
   * Used with `file_cipher` to decrypt the ciphertext chunks client-side.
   */
  key?: Uint8Array
}

export interface EncryptedAppLink {
  id: string
  file_id: string
  owner_id: string
  owner_email: string
  owner_pubkey: string
  owner_key_type?: string
  file_size: number
  file_mime: string
  signature: string
  downloads: number
  encrypted_name: string
  encrypted_link_key: string
  encrypted_file_key?: string

  /**
   * Only present on single-link metadata responses. The owner's link
   * listing clears it and sets `has_thumbnail` — the blob comes from
   * `GET /api/links/{id}/metadata` on demand.
   */
  encrypted_thumbnail?: string

  /** Whether the link carries a thumbnail, accurate even in listings. */
  has_thumbnail?: boolean

  created_at: number
  file_modified_at: number

  /** Cipher the file chunks were encrypted with (e.g. "aegis128l"). */
  file_cipher: string
  expires_at?: number
}

export interface EncryptedLink {
  id: string
  file_id: string
  user_id: string
  signature: string
  downloads: number
  encrypted_name: string
  encrypted_link_key: string
  encrypted_thumbnail?: string
  created_at: number
  expires_at?: number
  link_key_hex?: string
}
