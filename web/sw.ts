import { downloadAndDecryptStream } from './services/storage/workers'
import { FileMetadata } from './services/storage/metadata'
import { uploadFile } from './services/storage/workers/file'
import Api, { ErrorResponse, type ApiTransfer } from './services/api'
import * as cryptfns from './services/cryptfns'
import * as logger from '!/logger'

import type {
  DownloadCompletedResponseMessage,
  DownloadFileMessage,
  DownloadProgressResponseMessage,
  ListAppFile,
  UploadAppFile,
  UploadChunkResponseMessage,
  UploadFileMessage,
  WorkerErrorType
} from './types'

const sleep = (s: number) => new Promise((r) => setTimeout(r, s * 1000))

self.canceled = {
  upload: [],
  download: []
}

/**
 * Setup on the self object the API handler
 */
function handleApiTransfer(apiTransfer?: ApiTransfer) {
  if (apiTransfer && apiTransfer.apiUrl) {
    logger.debug('Setting the SWApi...', apiTransfer.apiUrl)

    const api = new Api(apiTransfer)
    self.SWApi = api
  } else {
    logger.warn('Missing apiTransfer...')
  }
}

onmessage = async (message: MessageEvent<any>) => {
  logger.debug('In worker, receiving message', message.data?.type || 'unknown')

  // Handle ping messages from the main thread
  if (message.data?.type === 'ping') {
    postMessage({ type: 'pong' })
  }

  // Crypto messages
  if (message.data?.type === 'encrypt' || message.data?.type === 'decrypt') {
    handleCrypto(
      message.data.type,
      message.data.message.id,
      message.data.message.data,
      message.data.message.key
    )
  }

  // Creating api maker with the updated credentials received
  // from the main browser thread that has access to JWT and CSRF
  if (message.data?.type === 'auth') {
    handleApiTransfer(message.data.apiTransfer)
  }

  if (message.data?.type === 'cancel') {
    const type = message.data.kind

    if (type === 'upload') {
      self.canceled.upload.push(message.data.id)
    } else if (type === 'download') {
      self.canceled.download.push(message.data.id)
    }
  }

  if (message.data?.type === 'upload-file') {
    handleApiTransfer(message.data.apiTransfer)

    while (!self.SWApi) {
      logger.warn('Waiting for SWApi to be initialized with credentials and start uploading...')
      await sleep(1)
    }

    handleUploadFile(message.data.message)
  }

  if (message.data?.type === 'download-file') {
    handleApiTransfer(message.data.apiTransfer)

    while (!self.SWApi) {
      logger.warn('Waiting for SWApi to be initialized with credentials and start downloading...')
      await sleep(1)
    }

    handleDownloadFile(message.data.message)
  }
}

/**
 * Handle crypto messages that either ask the worker to encrypt or decrypt
 * the data using the key provided.
 */
async function handleCrypto(
  type: 'encrypt' | 'decrypt',
  id: string,
  data: Uint8Array,
  key: Uint8Array
) {
  logger.debug('In worker, handling crypto', type, id)

  const fn = type === 'encrypt' ? cryptfns.aes.encrypt : cryptfns.aes.decrypt

  const result = await fn(data, key)

  logger.debug('In worker, handling crypto, posting message', type, id)

  postMessage({
    type,
    message: { id, result }
  })
}

/**
 * Handle taking the file in, chunking it and sending it to the backend
 */
async function handleUploadFile({
  transferableUploadedChunks,
  transferableFile,
  metadataJson
}: UploadFileMessage) {
  const file = transferableFile as UploadAppFile
  file.metadata = FileMetadata.fromJson(metadataJson)
  file.uploaded_chunks = transferableUploadedChunks as unknown as number[]

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
    await uploadFile(self.SWApi, file, progress)
  } catch (error) {
    logger.error('In worker error', error)
    progress(file, 0, false, error as ErrorResponse<unknown>)
  }
}

/**
 * Handle received download file message and downloading the file
 *
 * Once the download is generated it transfers the stream back
 * with the postMessage. This happens right away and
 */
async function handleDownloadFile({ transferableFile, metadataJson }: DownloadFileMessage) {
  transferableFile.metadata = FileMetadata.fromJson(metadataJson)

  try {
    const response = await downloadAndDecryptStream(
      self.SWApi,
      transferableFile,
      async (file: ListAppFile, chunkBytes: number): Promise<void> => {
        const transferableFile = { ...file, metadata: undefined }

        postMessage({
          type: 'download-progress',
          response: {
            transferableFile,
            metadataJson,
            chunkBytes
          } as DownloadProgressResponseMessage
        })
      }
    )

    postMessage({
      type: 'download-completed',
      response: {
        transferableFile,
        metadataJson,
        blob: await response.blob()
      } as DownloadCompletedResponseMessage
    })
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
