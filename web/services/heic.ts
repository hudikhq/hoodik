/**
 * Decode a HEIC/HEIF blob to a JPEG blob using libheif-js (WebAssembly libheif 1.19.x).
 * Throws if the file cannot be decoded.
 */
export async function heicToJpegBlob(blob: Blob, quality = 0.9): Promise<Blob> {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const libheif = (await import('libheif-js/wasm-bundle')).default as any
  const buffer = await blob.arrayBuffer()
  const decoder = new libheif.HeifDecoder()
  const images = decoder.decode(new Uint8Array(buffer))

  if (!images?.length) {
    throw new Error('Could not decode HEIC file')
  }

  const image = images[0]
  const width: number = image.get_width()
  const height: number = image.get_height()

  const canvas = document.createElement('canvas')
  canvas.width = width
  canvas.height = height
  const ctx = canvas.getContext('2d')!
  const imageData = ctx.createImageData(width, height)

  await new Promise<void>((resolve, reject) => {
    image.display(imageData, (result: ImageData | null) => {
      if (!result) {
        reject(new Error('HEIC display failed'))
        return
      }
      ctx.putImageData(imageData, 0, 0)
      resolve()
    })
  })

  return new Promise<Blob>((resolve, reject) =>
    canvas.toBlob(
      (b) => (b ? resolve(b) : reject(new Error('canvas.toBlob failed'))),
      'image/jpeg',
      quality
    )
  )
}
