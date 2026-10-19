<script setup lang="ts">
import { store as filesStore } from '!/storage'
import { store as queueStore } from '!/queue'
import { store as uploadStore } from '!/storage/upload'
import { store as downloadStore } from '!/storage/download'
import SingleFile from '@/components/files/io/SingleFile.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import { useKeyboardShortcuts } from '@/composables/useKeyboardShortcuts'
import { computed, ref, watch, onBeforeMount } from 'vue'
import { useI18n } from 'vue-i18n'
import { mdiChevronDown, mdiChevronUp } from '@mdi/js'
import type { DownloadAppFile, UploadAppFile, QueueItemActionType } from 'types'

type InnerFileList = {
  file: UploadAppFile | DownloadAppFile
  type: QueueItemActionType
}

const { t } = useI18n()

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

const MAX_QUEUE_ROWS = 100

type Bucket = {
  type: QueueItemActionType
  files: readonly (UploadAppFile | DownloadAppFile)[]
}

const buckets = computed<Bucket[]>(() => [
  { type: 'upload:running', files: upload.running },
  { type: 'upload:waiting', files: upload.waiting },
  { type: 'upload:done', files: upload.done },
  { type: 'upload:failed', files: upload.failed },
  { type: 'download:running', files: download.running },
  { type: 'download:waiting', files: download.waiting },
  { type: 'download:done', files: download.done },
  { type: 'download:failed', files: download.failed }
])

const totalItems = computed(
  () =>
    upload.running.length +
    upload.waiting.length +
    upload.done.length +
    upload.failed.length +
    download.running.length +
    download.waiting.length +
    download.done.length +
    download.failed.length
)
const activeItems = computed(() => upload.running.length + download.running.length)
const pendingItems = computed(() => upload.waiting.length + download.waiting.length)
const failedItems = computed(() => upload.failed.length + download.failed.length)
const doneItems = computed(() => upload.done.length + download.done.length)

const displayTotal = computed(() => {
  if (!showTable.value) return 0
  if (!tab.value) return totalItems.value

  return buckets.value
    .filter((bucket) => bucket.type.endsWith(tab.value as string))
    .reduce((sum, bucket) => sum + bucket.files.length, 0)
})

const displaying = computed((): InnerFileList[] => {
  if (!showTable.value) return []

  const list: InnerFileList[] = []

  for (const bucket of buckets.value) {
    if (tab.value && !bucket.type.endsWith(tab.value as string)) continue

    for (const file of bucket.files) {
      list.push({ type: bucket.type, file })
      if (list.length >= MAX_QUEUE_ROWS) return list
    }
  }

  return list
})

const hiddenItems = computed(() => Math.max(0, displayTotal.value - displaying.value.length))

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
  if (failed) return t('files.transfers.failedCount', { count: failed })
  if (doneItems.value) return t('files.transfers.doneCount', { count: doneItems.value })
  return ''
})

// The standalone count on the right mirrors `totalItems`. Hide it when the
// header label already contains the same number (e.g. "1 done" + "1" read
// as "1 done1" in a narrow collapsed popup) — only useful when there's a
// mix of states so total communicates more than the label alone.
const showStandaloneCount = computed(() => {
  const running = upload.running.length + download.running.length
  return running > 0 && totalItems.value > running
})

const currentTab = computed(() => tab.value ?? 'all')

const tabs = computed(() => [
  { key: 'all', label: t('files.transfers.all'), count: totalItems.value },
  { key: 'running', label: t('files.transfers.active'), count: activeItems.value },
  { key: 'waiting', label: t('files.transfers.pending'), count: pendingItems.value },
  { key: 'done', label: t('common.done'), count: doneItems.value },
  { key: 'failed', label: t('files.transfers.failed'), count: failedItems.value }
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

useKeyboardShortcuts([
  {
    key: 'Escape',
    inEditable: true,
    inDialog: true,
    preventDefault: false,
    when: () => showTable.value,
    handler: () => (showTable.value = false)
  }
])
</script>

<template>
  <div
    class="fixed bottom-0 right-0 z-[45] shadow-lg"
    :class="{ 'w-full xl:w-2/5': showTable }"
    style="padding-bottom: env(safe-area-inset-bottom, 0px)"
  >
    <!-- Sentinel for e2e tests: present while any upload/download is queued or running -->
    <span
      v-if="activeItems + pendingItems > 0"
      data-testid="upload-active"
      class="sr-only"
      aria-hidden="true"
    />

    <!-- Header bar -->
    <div
      class="cursor-pointer flex items-center justify-between px-3 py-2 select-none dark:text-white"
      @click="showTable = !showTable"
      :class="{
        'bg-redish-50 dark:bg-redish-700': totalItems > 0,
        'bg-paper-100 dark:bg-brownish-600': totalItems === 0
      }"
    >
      <span class="text-xs font-medium" aria-live="polite">{{ headerLabel || $t('files.transfers.title') }}</span>
      <div class="flex items-center gap-2">
        <span class="text-xs tabular-nums" v-if="showStandaloneCount">{{ totalItems }}</span>
        <BaseIcon :path="showTable ? mdiChevronDown : mdiChevronUp" w="w-4" h="h-4" />
      </div>
    </div>

    <!-- Tab bar + file list -->
    <div class="shadow rounded-sm" v-show="showTable">
      <!-- Scrollable pill tabs — overflow-x-auto so they scroll on narrow mobile screens -->
      <div
        class="flex overflow-x-auto gap-1 px-2 py-1.5 bg-paper-50 dark:bg-brownish-800 border-b border-paper-300 dark:border-brownish-700"
        style="-webkit-overflow-scrolling: touch; scrollbar-width: none"
      >
        <button
          v-for="t in tabs"
          :key="t.key"
          class="shrink-0 flex items-center gap-1 px-2.5 py-1 rounded-full text-xs font-medium transition-colors whitespace-nowrap"
          :class="
            currentTab === t.key
              ? 'bg-paper-300 dark:bg-brownish-600 text-brownish-900 dark:text-white'
              : 'text-brownish-500 dark:text-brownish-50 hover:bg-paper-200 dark:hover:bg-brownish-700'
          "
          @click="tab = t.key === 'all' ? undefined : (t.key as NonNullable<typeof tab>)"
        >
          {{ t.label }}
          <span class="tabular-nums opacity-70">{{ t.count }}</span>
        </button>
      </div>

      <!-- File list — viewport-capped height on mobile, fixed on larger screens -->
      <div class="max-h-[50vh] md:max-h-[325px] overflow-y-auto bg-paper-50 dark:bg-brownish-800">
        <template v-for="item in displaying" :key="`${item.file.id}-${item.type}`">
          <SingleFile :file="item.file" :type="item.type" @remove="remove" />
        </template>

        <div
          v-if="hiddenItems"
          class="text-center pb-2 pt-2 px-3 text-xs text-brownish-500 dark:text-brownish-50"
          data-testid="transfers-truncated"
        >
          {{
            $t('files.transfers.showingPartial', { shown: displaying.length, total: displayTotal })
          }}
        </div>

        <template v-if="!displaying.length">
          <div class="text-center pb-3 pt-4 text-sm text-brownish-500 dark:text-brownish-50">
            {{ $t('files.transfers.noActivity') }}
          </div>
        </template>
      </div>
    </div>
  </div>
</template>
