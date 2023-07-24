import type { Query } from '../services/api'
import type { store as downloadStore } from '../services/storage/download'
import type { store as uploadStore } from '../services/storage/upload'
import type { store as filesStore } from '../services/storage'
import type { store as queueStore } from '../services/queue'
import type { store as cryptoStore } from '../services/crypto'
import type { store as linksStore } from '../services/links'
import type { store as loginStore } from '../services/auth/login'
import type { AppFile, UploadAppFile, DownloadAppFile } from './file'

export * from './account'
export * from './admin'
export * from './create'
export * from './cryptfns'
export * from './file'
export * from './links'
export * from './login'
export * from './queue'
export * from './register'
export * from './worker'

export type UploadStore = ReturnType<typeof uploadStore>
export type DownloadStore = ReturnType<typeof downloadStore>
export type FilesStore = ReturnType<typeof filesStore>
export type QueueStore = ReturnType<typeof queueStore>
export type LoginStore = ReturnType<typeof loginStore>
export type CryptoStore = ReturnType<typeof cryptoStore>
export type LinksStore = ReturnType<typeof linksStore>

export interface Paginated<T> {
  data: T[]
  total: number
}

export interface Parameters extends Query {
  dir_id?: string | null
  order?: 'asc' | 'desc'
  order_by?: 'created_at' | 'size'
  dirs_only?: boolean
  is_owner?: boolean
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

export interface Stats {
  mime: string
  size: number
  count: number
}

export interface StorageStatsResponse {
  stats: Stats[]
  used_space: number
  quota?: number
}

export interface SingleChunk {
  data: Uint8Array
  chunk: number
}

export type IntervalType = ReturnType<typeof setInterval>

export type UploadProgressFunction = (file: UploadAppFile, done: boolean) => Promise<void>
export type DownloadProgressFunction = (file: AppFile, chunkBytes: number) => Promise<void>

export interface HelperType {
  decrypt(file: AppFile): Promise<AppFile>
  decrypt(file: AppFile): Promise<AppFile>
  decrypt(file: UploadAppFile): Promise<UploadAppFile>
  decrypt(file: DownloadAppFile): Promise<DownloadAppFile>
  [key: string]: any
}
