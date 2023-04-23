import type { Key } from '../cryptfns/aes'
import * as cryptfns from '../cryptfns'
import type { FileMetadataJson } from './types'

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
    let key

    if (this.key) {
      key = cryptfns.aes.keyToStringJson(this.key)
    }

    return cryptfns.rsa.encryptMessage(JSON.stringify({ ...this, key }), publicKey)
  }

  /**
   * Decrypt the file metadata from string
   */
  static async decrypt(encrypted: string, keypair: cryptfns.rsa.KeyPair): Promise<FileMetadata> {
    const decrypted = await cryptfns.rsa.decryptMessage(keypair, encrypted)

    try {
      const obj = JSON.parse(decrypted)
      let key = obj.key

      if (key) {
        try {
          key = cryptfns.aes.keyFromStringJson(obj.key)
        } catch (e) {
          obj.error =
            'Data key is not valid, most likely the file encryption was broken and it is unrecoverable'
        }
      }

      const metadata = new FileMetadata(obj.name, key)

      for (const key in obj) {
        if (key !== 'name' && key !== 'key') {
          metadata[key] = obj[key]
        }
      }

      return metadata
    } catch (error) {
      return new FileMetadata('decrypt failed', undefined)
    }
  }
}
