import { PREVIEW_MIME_TYPES, IMAGE_THUMBNAIL_SIZE_PX } from '../../constants'

/**
 * Take the selected file and create a thumbnail from it.
 * Uses URL.createObjectURL to avoid loading the entire file into memory.
 */
export function createThumbnail(file: File): Promise<string | undefined> {
  return new Promise((resolve) => {
    if (PREVIEW_MIME_TYPES.includes(file.type) === false) {
      return resolve(undefined)
    }

    const objectUrl = URL.createObjectURL(file)
    const image = new Image()

    image.onload = function () {
      const aspectRatio = image.width / image.height

      let thumbnailWidth = IMAGE_THUMBNAIL_SIZE_PX
      let thumbnailHeight = IMAGE_THUMBNAIL_SIZE_PX / aspectRatio

      if (aspectRatio < 1) {
        thumbnailWidth = IMAGE_THUMBNAIL_SIZE_PX * aspectRatio
        thumbnailHeight = IMAGE_THUMBNAIL_SIZE_PX
      }

      const canvas = document.createElement('canvas')
      canvas.width = thumbnailWidth
      canvas.height = thumbnailHeight

      const ctx = canvas.getContext('2d')
      ctx?.drawImage(image, 0, 0, image.width, image.height, 0, 0, canvas.width, canvas.height)

      const thumbnailData = canvas.toDataURL('image/jpeg')

      URL.revokeObjectURL(objectUrl)
      resolve(thumbnailData)
    }

    image.onerror = function () {
      URL.revokeObjectURL(objectUrl)
      resolve(undefined)
    }

    image.src = objectUrl
  })
}
