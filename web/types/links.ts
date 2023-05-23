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
}

export interface EncryptedAppLink {
  id: string
  file_id: string
  owner_id: string
  owner_email: string
  owner_pubkey: string
  file_size: number
  file_mime: string
  signature: string
  downloads: number
  encrypted_name: string
  encrypted_link_key: string
  encrypted_thumbnail?: string
  created_at: string
  file_created_at: string
  expires_at: string
}
