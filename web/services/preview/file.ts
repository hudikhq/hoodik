import { Preview } from '.'
import { store as FileStore } from '!/storage'
import type { AppFile, KeyPair } from 'types'
const store = FileStore()

export class FilePreview extends Preview implements Preview {
  private items?: FilePreview[]

  constructor(public file: AppFile, public keypair: KeyPair) {
    const { id, name, thumbnail, mime, size, file_id: parentId } = file

    super({ id, name, parentId, thumbnail, mime, size })

    this.file = file
    this.keypair = keypair
  }

  /**
   * Get the items that are in the same directory
   * as the main preview file (if applicable)
   */
  public async loadItems(): Promise<void> {
    if (this.items) {
      return
    }

    await store.find(this.keypair, this.file.file_id)

    this.items = store.items
      .map((i) => new FilePreview(i, this.keypair as KeyPair))
      .filter((i) => i.is())
  }

  /**
   * Get total number of items in the same directory
   */
  public getTotal(): number {
    return this.items?.length || 0
  }

  /**
   * Get index of the current preview item in the list of other
   * items from the same directory
   */
  public getIndex(): number {
    if (!this.items) {
      return -1
    }

    return this.items.findIndex((item) => item.id === this.id)
  }

  /**
   * Get the id of previous item
   */
  public getPreviousId(): string | undefined {
    const index = this.getIndex()
    const total = this.getTotal()

    if ((index === -1 && !total) || !this.items) return

    const previousIndex = index - 1

    if (previousIndex < 0) {
      return this.items[total - 1].id
    }

    return this.items[previousIndex].id
  }

  /**
   * Get the id of next item
   */
  public getNextId(): string | undefined {
    const index = this.getIndex()
    const total = this.getTotal()

    if ((index === -1 && !total) || !this.items) return

    const nextIndex = index + 1

    if (nextIndex >= total) {
      return this.items[0].id
    }

    return this.items[nextIndex].id
  }

  /**
   * Load the file data
   */
  public async load(): Promise<Uint8Array> {
    if (this.file.data) {
      return this.file.data
    }

    this.file = await store.get(this.file, this.keypair)

    if (!this.file.data) {
      throw new Error('Could not get file data')
    }

    return this.file.data
  }
}
