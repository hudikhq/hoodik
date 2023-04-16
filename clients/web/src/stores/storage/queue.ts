import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { AppFile } from './meta'

export interface UploadQueueItem extends Partial<AppFile> {
  file?: File
  name: string
  mime: string
  size: number
  chunks: number
  chunks_stored: number
  startedAt?: Date
  finishedAt?: Date
}

export const store = defineStore('upload-queue', () => {
  /**
   * Upload queue
   */
  const queue = ref<UploadQueueItem[]>([])

  /**
   * Number of files currently uploading
   */
  const uploading = computed(() => {
    return queue.value.filter((item) => item.startedAt && !item.finishedAt).length
  })

  /**
   * Add file to upload queue that will be picked up by the upload worker
   * and will start upload process async
   */
  function add(item: File) {
    queue.value.push({
      file: item,
      mime: item.type || 'text/plain',
      name: item.name,
      size: item.size,
      chunks: 0,
      chunks_stored: 0
    })
  }

  /**
   * Progress queue item status
   */
  function progress(item: UploadQueueItem, done?: boolean) {
    const index = queue.value.findIndex((i) => {
      if (item.id && i.id) {
        return i.id === item.id
      }

      return `${i.name}${i.size}` === `${item.name}${item.size}`
    })

    if (index === -1) {
      return
    }

    queue.value[index] = item
    queue.value[index].startedAt = queue.value[index].startedAt || new Date()

    if (done) {
      queue.value[index].finishedAt = new Date()
    }
  }

  return {
    queue,
    uploading,
    add,
    progress
  }
})
