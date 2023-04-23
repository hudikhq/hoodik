import { uploadChunk, download } from './stores/storage/workers'
import Api from './stores/api'

import type {
  DownloadFileMessage,
  DownloadProgressResponseMessage,
  UploadChunkMessage,
  UploadChunkResponseMessage
} from './stores/storage/types'
import type { ErrorResponse } from '@/stores/api'
import { FileMetadata } from './stores/storage/metadata'

onmessage = async (message: MessageEvent<any>) => {
  if (message.data?.type === 'upload-chunk') {
    handleUploadChunk(message.data.message)
  }

  if (message.data?.type === 'download-file') {
    handleDownloadFile(message.data.message)
  }
}

/**
 * Handle received upload chunk message
 */
async function handleUploadChunk({
  api,
  transferableFile,
  metadataJson,
  data,
  chunk,
  attempt
}: UploadChunkMessage) {
  try {
    transferableFile.metadata = FileMetadata.fromJson(metadataJson)

    const response = await uploadChunk(new Api(api), transferableFile, data, chunk, attempt || 0)

    console.log('Sending message back from worker')

    postMessage({ type: 'upload-progress', response })
  } catch (err) {
    const error = err as ErrorResponse<unknown>

    transferableFile = { ...transferableFile, metadata: undefined, file: undefined }
    const metadataJson = transferableFile.metadata?.toJson()

    postMessage({
      type: 'upload-progress',
      response: {
        transferableFile,
        metadataJson,
        data,
        chunk,
        attempt: attempt || 0,
        error: error.message || 'something went wrong'
      } as UploadChunkResponseMessage
    })
  }
}

/**
 * Handle received download file message and downloading the file
 */
async function handleDownloadFile({ api, transferableFile, metadataJson }: DownloadFileMessage) {
  transferableFile.metadata = FileMetadata.fromJson(metadataJson)

  try {
    await download(new Api(api), transferableFile)
  } catch (err) {
    const error = err as ErrorResponse<unknown>

    postMessage({
      type: 'download-progress',
      response: {
        transferableFile,
        chunkBytes: 0,
        error: error?.message || 'something went wrong'
      } as DownloadProgressResponseMessage
    })
  }
}
