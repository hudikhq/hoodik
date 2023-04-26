import { uploadChunk, download } from './stores/storage/workers'
import Api, { ErrorResponse } from './stores/api'
import { FileMetadata } from './stores/storage/metadata'
import { uploadFile } from './stores/storage/workers/file'

import type {
  DownloadFileMessage,
  DownloadProgressResponseMessage,
  UploadAppFile,
  UploadChunkMessage,
  UploadChunkResponseMessage,
  UploadFileMessage,
  WorkerErrorType
} from './stores/types'

onmessage = async (message: MessageEvent<any>) => {
  if (message.data?.type === 'upload-file') {
    handleUploadFile(message.data.message)
  }

  if (message.data?.type === 'upload-chunk') {
    handleUploadChunk(message.data.message)
  }

  if (message.data?.type === 'download-file') {
    handleDownloadFile(message.data.message)
  }

  if (message.data?.type === 'test') {
    console.log(message)
  }
}

/**
 * Handle taking the file in, chunking it and sending it to the backend
 */
async function handleUploadFile({ api, transferableFile, metadataJson }: UploadFileMessage) {
  const file = transferableFile as UploadAppFile
  file.metadata = FileMetadata.fromJson(metadataJson)

  const apiRunner = new Api(api)

  const progress = (
    file: UploadAppFile,
    attempt: number,
    isDone: boolean,
    error?: Error | ErrorResponse<any> | string | undefined
  ) => {
    const transferableFile = { ...file, metadata: undefined }

    postMessage({
      type: 'upload-progress',
      response: {
        transferableFile,
        metadataJson,
        attempt: attempt || 0,
        isDone,
        error: handleError(error)
      } as UploadChunkResponseMessage
    })
  }

  try {
    await uploadFile(apiRunner, file, progress)
  } catch (error) {
    console.log('In worker error')
    progress(file, 0, false, error as ErrorResponse<unknown>)
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

    transferableFile = { ...transferableFile, metadata: undefined }
    const metadataJson = transferableFile.metadata?.toJson()

    postMessage({
      type: 'upload-progress',
      response: {
        transferableFile,
        metadataJson,
        data,
        chunk,
        attempt: attempt || 0,
        error: handleError(error)
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
        error: handleError(error)
      } as DownloadProgressResponseMessage
    })
  }
}

/**
 * Convert error into something receivable by the main thread
 */
function handleError(error?: undefined | Error | ErrorResponse<any> | string): WorkerErrorType {
  if (!error) return

  if (typeof error === 'string') {
    return { context: error }
  }

  if (error instanceof Error) {
    return {
      context:
        (error as ErrorResponse<unknown>).validation ||
        (error as ErrorResponse<unknown>).description ||
        error.message,
      stack: error.stack
    }
  }
}
