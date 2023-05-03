import type { Query } from '../services/api'
import type { store as downloadStore } from '../services/storage/download'
import type { store as uploadStore } from '../services/storage/upload'
import type { store as filesStore } from '../services/storage'
import type { store as queueStore } from '../services/queue'
import type { AppFile, ListAppFile, UploadAppFile, DownloadAppFile } from './file'

export * from './create'
export * from './file'
export * from './worker'
export * from './queue'
export * from './login'
export * from './register'
export * from './cryptfns'

export type UploadStore = ReturnType<typeof uploadStore>
export type DownloadStore = ReturnType<typeof downloadStore>
export type FilesStore = ReturnType<typeof filesStore>
export type QueueStore = ReturnType<typeof queueStore>

export interface Parameters extends Query {
  dir_id?: string | null
  order?: 'asc' | 'desc'
  order_by?: 'created_at' | 'size'
}

export interface SearchQuery {
  search_tokens_hashed: string[]
  dir_id?: string
  limit?: number
  skip?: number
}

export interface FileResponse {
  parents?: AppFile[]
  children: AppFile[]
}

export interface SingleChunk {
  data: Uint8Array
  chunk: number
}

export type IntervalType = ReturnType<typeof setInterval>

export type UploadProgressFunction = (file: UploadAppFile, done: boolean) => Promise<void>
export type DownloadProgressFunction = (file: ListAppFile, chunkBytes: number) => Promise<void>

export interface FileMetadataJson {
  name?: string
  key?: string
  [other: string]: any
}

export interface HelperType {
  decrypt(file: AppFile): Promise<AppFile>
  decrypt(file: ListAppFile): Promise<ListAppFile>
  decrypt(file: UploadAppFile): Promise<UploadAppFile>
  decrypt(file: DownloadAppFile): Promise<DownloadAppFile>
  [key: string]: any
}
