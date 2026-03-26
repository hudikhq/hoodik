<script setup lang="ts">
import { store as filesStore } from '!/storage'
import { store as queueStore } from '!/queue'
import { store as uploadStore } from '!/storage/upload'
import { store as downloadStore } from '!/storage/download'
import SingleFile from '@/components/files/io/SingleFile.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import { computed, ref, watch, onBeforeMount } from 'vue'
import { mdiChevronDown, mdiChevronUp } from '@mdi/js'
import type { DownloadAppFile, UploadAppFile, QueueItemActionType } from 'types'

type InnerFileList = {
  file: UploadAppFile | DownloadAppFile
  type: QueueItemActionType
}

const queue = queueStore()
const files = filesStore()
const upload = uploadStore()
const download = downloadStore()

onBeforeMount(async () => {
  await queue.start(files, upload, download)
})

const showTable = ref(false)
const showedTable = ref(false)
const tab = ref<'running' | 'done' | 'waiting' | 'failed'>()

const items = computed((): InnerFileList[] => {
  const list = [
    ...upload.running.map((item: UploadAppFile) => ({
      type: 'upload:running' as QueueItemActionType,
      file: item
    })),
    ...upload.waiting.map((item: UploadAppFile) => ({
      type: 'upload:waiting' as QueueItemActionType,
      file: item
    })),
    ...upload.done.map((item: UploadAppFile) => ({
      type: 'upload:done' as QueueItemActionType,
      file: item
    })),
    ...upload.failed.map((item: UploadAppFile) => ({
      type: 'upload:failed' as QueueItemActionType,
      file: item
    })),
    ...download.running.map((item: DownloadAppFile) => ({
      type: 'download:running' as QueueItemActionType,
      file: item
    })),
    ...download.waiting.map((item: DownloadAppFile) => ({
      type: 'download:waiting' as QueueItemActionType,
      file: item
    })),
    ...download.done.map((item: DownloadAppFile) => ({
      type: 'download:done' as QueueItemActionType,
      file: item
    })),
    ...download.failed.map((item: DownloadAppFile) => ({
      type: 'download:failed' as QueueItemActionType,
      file: item
    }))
  ]

  list.sort((a, b) => {
    if (a.type.endsWith('running')) return -1
    if (b.type.endsWith('running')) return 1
    return 0
  })

  return list
})

const totalItems = computed(() => items.value.length)
const activeItems = computed(() => items.value.filter((i) => i.type.endsWith('running')).length)
const pendingItems = computed(() => items.value.filter((i) => i.type.endsWith('waiting')).length)
const failedItems = computed(() => items.value.filter((i) => i.type.endsWith('failed')).length)
const doneItems = computed(() => items.value.filter((i) => i.type.endsWith('done')).length)

const displaying = computed((): InnerFileList[] => {
  if (!tab.value) return items.value
  return items.value.filter((item) => item.type.endsWith(tab.value as string))
})

const headerLabel = computed(() => {
  const up = upload.running.length
  const dl = download.running.length
  const failed = upload.failed.length + download.failed.length
  if (up || dl) {
    const parts: string[] = []
    if (up) parts.push(`↑ ${up}`)
    if (dl) parts.push(`↓ ${dl}`)
    return parts.join(' · ')
  }
  if (failed) return `${failed} failed`
  if (doneItems.value) return `${doneItems.value} done`
  return ''
})

const currentTab = computed(() => tab.value ?? 'all')

const tabs = computed(() => [
  { key: 'all', label: 'All', count: totalItems.value },
  { key: 'running', label: 'Active', count: activeItems.value },
  { key: 'waiting', label: 'Pending', count: pendingItems.value },
  { key: 'done', label: 'Done', count: doneItems.value },
  { key: 'failed', label: 'Failed', count: failedItems.value }
])

watch(
  () => totalItems.value,
  () => {
    if (!showTable.value && !showedTable.value) {
      showTable.value = true
      showedTable.value = true
    }
  }
)

const remove = (file: UploadAppFile, type: QueueItemActionType) => {
  if (type === 'upload:running') {
    return upload.cancel(files, file)
  }

  if (type === 'upload:waiting') {
    upload.waiting = upload.waiting.filter((item) => item.id !== file.id)
  }

  if (type === 'upload:done') {
    upload.done = upload.done.filter((item) => item.id !== file.id)
  }

  if (type === 'upload:failed') {
    upload.failed = upload.failed.filter((item) => item.id !== file.id)
  }

  if (type === 'download:running') {
    return download.cancel(files, file)
  }

  if (type === 'download:waiting') {
    download.waiting = download.waiting.filter((item) => item.id !== file.id)
  }

  if (type === 'download:done') {
    download.done = download.done.filter((item) => item.id !== file.id)
  }

  if (type === 'download:failed') {
    download.failed = download.failed.filter((item) => item.id !== file.id)
  }
}

window.addEventListener('keydown', (e) => {
  if (e.key === 'Escape' && showTable.value) {
    showTable.value = false
  }
})
</script>

<template>
  <div
    class="fixed bottom-0 right-0 z-[45] shadow-lg"
    :class="{ 'w-full xl:w-2/5': showTable }"
    style="padding-bottom: env(safe-area-inset-bottom, 0px)"
  >
    <!-- Sentinel for e2e tests: present while any upload/download is actively running -->
    <span v-if="activeItems > 0" data-testid="upload-active" class="sr-only" aria-hidden="true" />

    <!-- Header bar -->
    <div
      class="cursor-pointer flex items-center justify-between px-3 py-2 select-none dark:text-white"
      @click="showTable = !showTable"
      :class="{
        'bg-redish-50 dark:bg-redish-700': totalItems > 0,
        'bg-brownish-100 dark:bg-brownish-600': totalItems === 0
      }"
    >
      <span class="text-xs font-medium">{{ headerLabel || 'Transfers' }}</span>
      <div class="flex items-center gap-2">
        <span class="text-xs tabular-nums" v-if="totalItems">{{ totalItems }}</span>
        <BaseIcon :path="showTable ? mdiChevronDown : mdiChevronUp" w="w-4" h="h-4" />
      </div>
    </div>

    <!-- Tab bar + file list -->
    <div class="shadow rounded-sm" v-show="showTable">
      <!-- Scrollable pill tabs — overflow-x-auto so they scroll on narrow mobile screens -->
      <div
        class="flex overflow-x-auto gap-1 px-2 py-1.5 bg-brownish-50 dark:bg-brownish-800 border-b border-brownish-200 dark:border-brownish-700"
        style="-webkit-overflow-scrolling: touch; scrollbar-width: none"
      >
        <button
          v-for="t in tabs"
          :key="t.key"
          class="shrink-0 flex items-center gap-1 px-2.5 py-1 rounded-full text-xs font-medium transition-colors whitespace-nowrap"
          :class="
            currentTab === t.key
              ? 'bg-brownish-300 dark:bg-brownish-600 text-brownish-900 dark:text-white'
              : 'text-brownish-500 dark:text-brownish-400 hover:bg-brownish-200 dark:hover:bg-brownish-700'
          "
          @click="tab = t.key === 'all' ? undefined : (t.key as typeof tab.value)"
        >
          {{ t.label }}
          <span class="tabular-nums opacity-70">{{ t.count }}</span>
        </button>
      </div>

      <!-- File list — viewport-capped height on mobile, fixed on larger screens -->
      <div class="max-h-[50vh] md:max-h-[325px] overflow-y-auto bg-brownish-50 dark:bg-brownish-800">
        <template v-for="item in displaying" :key="`${item.file.id}-${item.type}`">
          <SingleFile :file="item.file" :type="item.type" @remove="remove" />
        </template>

        <template v-if="!displaying.length">
          <div class="text-center pb-3 pt-4 text-sm text-brownish-500 dark:text-brownish-400">
            No activity in progress
          </div>
        </template>
      </div>
    </div>
  </div>
</template>
