import Api, { type ErrorResponse } from '@/stores/api'
import { sha256 } from '../cryptfns'

export const CHUNK_SIZE_BYTES = 1024 * 1024
export const MAX_RETRIES = 3

export interface CreateFile {
  name_enc: string
  search_tokens_hashed?: string[]
  encrypted_key: string
  checksum: string
  mime: string
  size?: number
  chunks?: number
  file_id?: number
  file_created_at?: string
}

export interface AppFile {
  id: number
  user_id: number
  name_enc: String
  checksum: String
  encrypted_key: String
  mime: String
  size: number
  chunks: number
  chunks_stored: number
  file_id?: number
  file_created_at: string
  created_at: string
  finished_upload_at?: string
  is_new: boolean
}

/**
 * Create or get a file before starting the upload
 */
export async function getOrCreate(file: CreateFile): Promise<AppFile> {
  const response = await Api.post<CreateFile, AppFile>('/api/storage', undefined, file)

  if (!response?.body?.id) {
    throw new Error('Failed to create file')
  }

  return response.body
}

/**
 * Upload a file chunk
 */
export async function uploadChunk(
  file: AppFile,
  data: Buffer,
  chunk?: number,
  attempt: number = 0
): Promise<AppFile> {
  const query = {
    chunk: chunk || file.chunks_stored,
    checksum: sha256.digest(data)
  }

  const headers = {
    'Content-Type': 'application/octet-stream',
    'Content-Length': data.length.toString()
  }

  try {
    const response = await Api.post<Buffer, AppFile>(
      `/api/storage/${file.id}`,
      query,
      data,
      headers
    )
    return response.body as AppFile
  } catch (err) {
    const error = err as ErrorResponse<Buffer>

    // If we get checksum error, most likely the data was corrupted during transfer
    // we wont retry indefinitely, but we will try a few times
    if (error.validation?.checksum && attempt < MAX_RETRIES) {
      return uploadChunk(file, data, chunk, attempt + 1)
    }

    // The chunk was already uploaded, so we can just return the file
    if (error.validation?.chunk === 'chunk_already_exists') {
      return file
    }

    throw err
  }
}

/**
 * Upload a whole file in chunks
 */
export async function uploadFile(file: AppFile, plainTextData: Buffer): Promise<AppFile> {
  const chunks = []
  let offset = 0

  while (offset < plainTextData.length) {
    const chunkSize = Math.min(CHUNK_SIZE_BYTES, plainTextData.length - offset)
    const chunkBuffer = plainTextData.slice(offset, offset + chunkSize)
    chunks.push(chunkBuffer)
    offset += chunkSize
  }

  const files = await Promise.all(
    chunks.map((chunkBuffer, index) => uploadChunk(file, chunkBuffer, index))
  )

  return files[files.length - 1]
}
