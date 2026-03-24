import { PREVIEW_MIME_TYPES, IMAGE_THUMBNAIL_SIZE_PX } from '../../constants'
import { heicToJpegBlob } from '../../heic'

const HEIC_MIME_TYPES = ['image/heic', 'image/heif']

/**
 * Take the selected file and create a thumbnail from it.
 * HEIC/HEIF files are converted to JPEG first via libheif-js.
 */
export async function createThumbnail(file: File): Promise<string | undefined> {
  if (!PREVIEW_MIME_TYPES.includes(file.type)) {
    return undefined
  }

  let blob: Blob = file

  if (HEIC_MIME_TYPES.includes(file.type)) {
    try {
      blob = await heicToJpegBlob(file, 0.8)
    } catch {
      return undefined
    }
  }

  return new Promise((resolve) => {
    const objectUrl = URL.createObjectURL(blob)
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
