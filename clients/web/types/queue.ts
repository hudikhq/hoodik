import type { AppFile } from './file'

export interface QueueItem extends Partial<AppFile> {
  file?: File
  name: string
  mime: string
  size: number
  chunks: number
  chunks_stored: number
  startedAt?: Date
  finishedAt?: Date
  type: 'upload' | 'download'
}

export type QueueItemActionType =
  | 'upload:running'
  | 'upload:waiting'
  | 'upload:failed'
  | 'upload:done'
  | 'download:running'
  | 'download:waiting'
  | 'download:failed'
  | 'download:done'
