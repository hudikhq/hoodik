import type {
  FilesStore,
  DownloadAppFile,
  UploadAppFile,
  AppFile,
  HelperType,
  KeyPair,
  EncryptedAppFile
} from 'types'
import * as meta from './meta'

/**
 * Helper class that can be easily shared between components to enable common requirements
 * and unified way of doing things using the stores (without having to invoke them directly
 * in each and every component)
 */
export class Helper implements HelperType {
  keypair: KeyPair
  store: FilesStore

  constructor(keypair: KeyPair, store: FilesStore) {
    this.keypair = keypair
    this.store = store
  }

  /**
   * Easily decrypt a file and replace it in the store
   * if it already wasn't decrypted.
   */
  async decrypt(file: EncryptedAppFile): Promise<EncryptedAppFile>
  async decrypt(file: AppFile): Promise<AppFile>
  async decrypt(file: UploadAppFile): Promise<UploadAppFile>
  async decrypt(file: DownloadAppFile): Promise<DownloadAppFile> {
    if (!this.keypair.input) {
      throw new Error('Cannot decrypt without private key')
    }

    if (!file.key) {
      const decrypted = await meta.decrypt(file, this.keypair.input)

      return {
        ...file,
        ...decrypted
      }
    }

    return file
  }
}
