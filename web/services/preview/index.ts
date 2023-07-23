type NonFunctionPropertyNames<T> = { [K in keyof T]: T[K] extends Function ? never : K }[keyof T]
export type ConstructPreview<T> = Pick<T, NonFunctionPropertyNames<T>>

/**
 * Base class for previews, contains common properties
 * used for preview of any type of inner file definition (file or link).
 * It also exposes static methods for creating previews from file or link.
 *
 * Each link, or file preview exposes inner functions that provide the actual
 * data for the preview and other actions that can be called on each respectively.
 *
 * @abstract
 */
export abstract class Preview {
  public id: string
  public parentId?: string
  public name: string
  public mime: string
  public size: number | undefined
  public thumbnail?: string

  constructor(data: ConstructPreview<Preview>) {
    this.id = data.id
    this.name = data.name
    this.mime = data.mime
    this.size = data.size
    this.parentId = data.parentId
    this.thumbnail = data.thumbnail
  }

  /**
   * Get the items that are in the same directory
   * as the main preview file (if applicable)
   */
  public async loadItems(): Promise<void> {
    return
  }

  /**
   * Get total number of items in the same directory
   */
  public getTotal(): number {
    return 0
  }

  /**
   * Get index of the current preview item in the list of other
   * items from the same directory
   */
  public getIndex(): number {
    return -1
  }

  /**
   * Get the id of previous item
   */
  public getPreviousId(): string | undefined {
    return undefined
  }

  /**
   * Get the id of next item
   */
  public getNextId(): string | undefined {
    return undefined
  }

  /**
   * Load the data for the preview
   */
  public async load(): Promise<Uint8Array> {
    return new Uint8Array()
  }

  /**
   * Easily match the preview type
   */
  public previewType(): 'image' | 'pdf' | 'video' | null {
    if (this.isImage()) {
      return 'image'
    }

    if (this.isPdf()) {
      return 'pdf'
    }

    // TODO: maybe someday
    // if (this.isVideo()) {
    //   return 'video'
    // }

    return null
  }

  /**
   * Lets us know if the wrapped file can have a preview at all
   */
  public is(): boolean {
    return (
      !!this.size &&
      this.size > 0 &&
      (this.thumbnail !== undefined || this.mime === 'application/pdf')
    )
  }

  /**
   * Lets us know if the preview is an image
   */
  public isImage(): boolean {
    return this.is() && this.thumbnail !== undefined && this.mime.startsWith('image/')
  }

  /**
   * Lets us know if the preview is a pdf file
   */
  public isPdf(): boolean {
    return this.is() && this.mime === 'application/pdf'
  }

  /**
   * Lets us know if the preview is a video
   */
  public isVideo(): boolean {
    return this.is() && this.thumbnail !== undefined && this.mime.startsWith('video/')
  }
}
