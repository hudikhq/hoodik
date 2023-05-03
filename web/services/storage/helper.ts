import type {
  FilesStore,
  ListAppFile,
  DownloadAppFile,
  UploadAppFile,
  AppFile,
  HelperType,
  KeyPair
} from 'types'
import { FileMetadata } from './metadata'

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
  async decrypt(file: AppFile): Promise<AppFile>
  async decrypt(file: ListAppFile): Promise<ListAppFile>
  async decrypt(file: UploadAppFile): Promise<UploadAppFile>
  async decrypt(file: DownloadAppFile): Promise<DownloadAppFile> {
    if (!file.metadata || !file.metadata.key) {
      const metadata = await FileMetadata.decrypt(file.encrypted_metadata, this.keypair)
      this.store.updateItem({ ...file, metadata })

      return { ...file, metadata }
    }

    return file
  }
}
