import * as cryptfns from '../cryptfns'
import type { FileMetadataJson, Key, KeyPair } from 'types'

export class FileMetadata {
  /**
   * File name
   */
  name?: string

  /**
   * AES key used to encrypt the file data
   */
  key?: Key;

  /**
   * Place to store any other possible metadata
   */
  [other: string]: any

  constructor(name: string, key: Key | undefined) {
    this.name = name
    this.key = key
  }

  /**
   * Convert this object into a JSON object
   */
  toJson(): FileMetadataJson {
    const obj = { name: this.name }

    for (const key in this) {
      if (typeof key === 'string' && key !== 'name' && key !== 'key' && typeof key !== 'function') {
        // @ts-ignore
        obj[key] = this[key]
      }
    }

    if (this.key) {
      // @ts-ignore - we are forcing key to be string here.
      obj.key = cryptfns.aes.keyToStringJson(this.key)
    }

    return obj
  }

  /**
   * Convert this to string
   */
  toString(): string {
    return JSON.stringify(this.toJson())
  }

  /**
   * Convert back from JSON
   */
  static fromJson(obj: { name?: string; key?: string; [other: string]: any } | null): FileMetadata {
    obj = obj || {}
    const { key, ...rest } = obj

    let uint8Key

    if (key) {
      uint8Key = cryptfns.aes.keyFromStringJson(key)
    }

    const metadata = new FileMetadata(obj.name || '', uint8Key)

    for (const key in rest) {
      metadata[key] = rest[key]
    }

    return metadata
  }

  /**
   * Convert back from string
   */
  static fromString(str: string): FileMetadata {
    return this.fromJson(JSON.parse(str))
  }

  /**
   * Set any additional information about the file
   */
  setExtras(extras: { [key: string]: string | null | undefined }): FileMetadata {
    for (const key in extras) {
      if (extras[key] !== undefined && extras[key] !== null) {
        this[key] = extras[key]
      }
    }

    return this
  }

  /**
   * Set any other possible arguments of the metadata
   */
  set(key: string, value: any): void {
    if (key !== 'name' && key !== 'key') {
      this[key] = value
    }
  }

  /**
   * Return encrypted string of the file metadata
   */
  async encrypt(publicKey: string): Promise<string> {
    if (!this.key) {
      throw new Error('Cannot encrypt without a file key')
    }

    const encrypted = this.toJson()

    for (const key in encrypted) {
      if (key !== 'key' && encrypted[key] !== undefined && encrypted[key] !== null) {
        encrypted[key] = await cryptfns.aes.encryptString(encrypted[key], this.key)
      }
    }

    // Only the AES key is encrypted with RSA public key
    if (encrypted.key) {
      encrypted.key = await cryptfns.rsa.encryptMessage(encrypted.key, publicKey)
    }

    return JSON.stringify(encrypted)
  }

  /**
   * Decrypt the file metadata from string
   */
  static async decrypt(encryptedString: string, keypair: KeyPair): Promise<FileMetadata> {
    try {
      const encrypted = JSON.parse(encryptedString)

      if (!encrypted.key) {
        throw new Error('Cannot decrypt without a file key')
      }

      if (!keypair.input) {
        throw new Error('Cannot decrypt without a private RSA key')
      }

      const encryptedAesKey = encrypted.key
      const decryptedAesKey = await cryptfns.rsa.decryptMessage(keypair, encryptedAesKey)
      const key = cryptfns.aes.keyFromStringJson(decryptedAesKey)

      // Decrypt the AES key with RSA private key
      const decrypted: FileMetadataJson = {}

      // Decrypt the rest of the metadata with AES key
      for (const k in encrypted) {
        if (k !== 'key' && encrypted[k] !== undefined && encrypted[k] !== null) {
          decrypted[k] = await cryptfns.aes.decryptString(encrypted[k], key)
        }
      }

      return new FileMetadata(decrypted.name || '', key).setExtras(decrypted)
    } catch (error) {
      return new FileMetadata((error as Error).message, undefined)
    }
  }
}
