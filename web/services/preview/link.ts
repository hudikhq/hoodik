import type { AppLink } from 'types'
import { Preview } from '.'
import { store as LinkStore } from '!/links'

const store = LinkStore()

export class LinkPreview extends Preview {
  private data?: Uint8Array

  /**
   * Create preview from link
   */
  constructor(private link: AppLink) {
    const { file_id: id, name, thumbnail, file_mime: mime, file_size: size } = link

    super({ id, name, thumbnail, mime, size, editable: false })
  }

  /**
   * Load the file data (client-side decrypt of raw ciphertext from server).
   */
  public async load(): Promise<Uint8Array> {
    if (this.data) {
      return this.data
    }

    this.data = await store.download(this.link.id, this.link.link_key_hex || '')
    return this.data
  }
}
