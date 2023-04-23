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
