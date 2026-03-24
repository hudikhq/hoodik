import { PREVIEW_MIME_TYPES, IMAGE_THUMBNAIL_SIZE_PX, VIDEO_MIME_TYPES, VIDEO_THUMBNAIL_SIZE_PX } from '../../constants'
import { heicToJpegBlob } from '../../heic'

const HEIC_MIME_TYPES = ['image/heic', 'image/heif']

/**
 * Take the selected file and create a thumbnail from it.
 * HEIC/HEIF files are converted to JPEG first via libheif-js.
 * Video files have a frame captured at ~1 second.
 */
export async function createThumbnail(file: File): Promise<string | undefined> {
  if (VIDEO_MIME_TYPES.includes(file.type)) {
    return createVideoThumbnail(file)
  }

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

/**
 * Capture a single video frame at ~1 second (or 10% of duration if shorter)
 * and return it as a base64 JPEG data URL. Resolves to undefined on any failure.
 */
async function createVideoThumbnail(file: File): Promise<string | undefined> {
  return new Promise((resolve) => {
    const objectUrl = URL.createObjectURL(file)
    const video = document.createElement('video')
    video.preload = 'metadata'
    video.muted = true
    video.playsInline = true

    video.onloadedmetadata = () => {
      video.currentTime = Math.min(1, video.duration * 0.1)
    }

    video.onseeked = () => {
      const aspectRatio = video.videoWidth / video.videoHeight

      let thumbW = VIDEO_THUMBNAIL_SIZE_PX
      let thumbH = VIDEO_THUMBNAIL_SIZE_PX / aspectRatio

      if (aspectRatio < 1) {
        thumbW = VIDEO_THUMBNAIL_SIZE_PX * aspectRatio
        thumbH = VIDEO_THUMBNAIL_SIZE_PX
      }

      const canvas = document.createElement('canvas')
      canvas.width = thumbW
      canvas.height = thumbH
      canvas.getContext('2d')?.drawImage(video, 0, 0, canvas.width, canvas.height)

      URL.revokeObjectURL(objectUrl)
      resolve(canvas.toDataURL('image/jpeg'))
    }

    video.onerror = () => {
      URL.revokeObjectURL(objectUrl)
      resolve(undefined)
    }

    video.src = objectUrl
  })
}
