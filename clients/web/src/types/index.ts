import type { Query } from '../stores/api'
import type { store as downloadStore } from '../stores/storage/download'
import type { store as uploadStore } from '../stores/storage/upload'
import type { store as filesStore } from '../stores/storage'
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
