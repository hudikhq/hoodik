import { Preview } from '.'
import { store as FileStore } from '!/storage'
import { downloadChunk } from '!/storage/download/sync'
import type { AppFile, KeyPair } from 'types'
const store = FileStore()

export class FilePreview extends Preview implements Preview {
  private items?: FilePreview[]

  constructor(public file: AppFile, public keypair: KeyPair) {
    const { id, name, thumbnail, mime, size, file_id: parentId, editable } = file

    super({ id, name, parentId, thumbnail, mime, size, editable })

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

  public get chunks(): number | undefined {
    return this.file.chunks
  }

  public async loadChunk(index: number, signal?: AbortSignal): Promise<Uint8Array> {
    return downloadChunk(this.file, index, signal)
  }

  /**
   * Load the file data.
   *
   * The cached `this.file.data` short-circuits re-downloads for the
   * same FilePreview instance, but edits (version restore, fork,
   * concurrent save) change what "the current file content" points at
   * while the FilePreview stays the same object. Callers that know a
   * refresh is needed should call [[invalidate]] first.
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

  /**
   * Drop the cached decrypted content. The next `load()` re-downloads
   * the chunks — used after restore/fork so the editor picks up the
   * newly-flipped active version.
   */
  public invalidate(): void {
    this.file.data = undefined
  }

  /**
   * Swap the wrapped file for a fresher snapshot (e.g. post-restore
   * payload the server returned). Preserves the preview's identity and
   * forces a re-download on the next `load()`.
   */
  public updateFile(file: AppFile): void {
    // Preserve the symmetric key — it's set by the caller on the
    // original metadata fetch and the restore response drops it.
    this.file = { ...this.file, ...file, key: file.key || this.file.key, data: undefined }
  }
}
