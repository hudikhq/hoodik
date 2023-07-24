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

    super({ id, name, thumbnail, mime, size })
  }

  /**
   * Load the file data
   */
  public async load(): Promise<Uint8Array> {
    if (this.data) {
      return this.data
    }

    const response = await store.download(this.link.id, this.link.link_key_hex)

    if (!response.ok) {
      throw new Error('Could not get file data')
    }

    const data = await response.arrayBuffer()

    this.data = new Uint8Array(data)

    return this.data
  }
}
