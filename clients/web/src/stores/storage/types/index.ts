import type { Query } from '../../api'
import type { store as downloadStore } from '../download'
import type { store as uploadStore } from '../upload'
import type { store as filesStore } from '..'
import type { AppFile, ListAppFile, UploadAppFile } from './file'

export * from './create'
export * from './file'
export * from './worker'

export type UploadStore = ReturnType<typeof uploadStore>
export type DownloadStore = ReturnType<typeof downloadStore>
export type FilesStore = ReturnType<typeof filesStore>

export interface Parameters extends Query {
  dir_id?: number | null
  order?: 'asc' | 'desc'
  order_by?: 'created_at' | 'size'
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
