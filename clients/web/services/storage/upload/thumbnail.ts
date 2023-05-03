import { PREVIEW_MIME_TYPES, IMAGE_THUMBNAIL_SIZE_PX } from '../constants'

/**
 * Take the selected file and create a thumbnail from it
 */
export function createThumbnail(file: File): Promise<string | null> {
  return new Promise((resolve, reject) => {
    if (PREVIEW_MIME_TYPES.includes(file.type) === false) {
      return resolve(null)
    }

    const reader = new FileReader()

    reader.onerror = reject

    reader.onloadend = function () {
      const canvas = document.createElement('canvas')
      const ctx = canvas.getContext('2d')

      const image = new Image()

      // set the onload event of the Image object
      image.onload = function () {
        // calculate the aspect ratio of the image
        const aspectRatio = image.width / image.height

        // initialize the thumbnail dimensions
        let thumbnailWidth = IMAGE_THUMBNAIL_SIZE_PX
        let thumbnailHeight = IMAGE_THUMBNAIL_SIZE_PX / aspectRatio

        // if the image is taller than wide, use the height to scale the thumbnail
        if (aspectRatio < 1) {
          thumbnailWidth = IMAGE_THUMBNAIL_SIZE_PX * aspectRatio
          thumbnailHeight = IMAGE_THUMBNAIL_SIZE_PX
        }

        canvas.width = thumbnailWidth
        canvas.height = thumbnailHeight

        // draw the loaded image onto the canvas
        ctx?.drawImage(image, 0, 0, image.width, image.height, 0, 0, canvas.width, canvas.height)

        // get the base64 encoded data of the canvas image
        const thumbnailData = canvas.toDataURL('image/jpeg')

        resolve(thumbnailData)
      }

      // set the source of the Image object to the data URL of the uploaded file
      image.src = reader.result as string
    }

    // read the selected file as a data URL
    reader.readAsDataURL(file)
  })
}
