<script setup lang="ts">
import { store as filesStore } from '!/storage'
import { store as queueStore } from '!/storage/queue'
import { store as uploadStore } from '!/storage/upload'
import { store as downloadStore } from '!/storage/download'
import SingleFile from '@/components/files/io/SingleFile.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import { computed, ref, watch, onBeforeMount } from 'vue'
import { mdiChevronDown, mdiChevronUp } from '@mdi/js'
import type { DownloadAppFile, UploadAppFile, QueueItemActionType } from 'types'
import BaseButton from '@/components/ui/BaseButton.vue'

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
  const items = [
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

  items.sort((a, b) => {
    if (a.type.endsWith('running')) {
      return -1
    }

    if (b.type.endsWith('running')) {
      return 1
    }

    return 0
  })

  return items
})

const totalItems = computed(() => {
  return items.value.length
})

const activeItems = computed(() => {
  return items.value.filter((i) => i.type.endsWith('running')).length
})

const pendingItems = computed(() => {
  return items.value.filter((i) => i.type.endsWith('waiting')).length
})

const failedItems = computed(() => {
  return items.value.filter((i) => i.type.endsWith('failed')).length
})

const doneItems = computed(() => {
  return items.value.filter((i) => i.type.endsWith('done')).length
})

const displaying = computed((): InnerFileList[] => {
  if (!tab.value) {
    return items.value
  }

  return items.value.filter((item) => item.type.endsWith(tab.value as string))
})

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

  if (type === 'download:done') {
    download.done = download.done.filter((item) => item.id !== file.id)
  }

  if (type === 'download:done') {
    download.done = download.done.filter((item) => item.id !== file.id)
  }

  if (type === 'download:failed') {
    download.failed = download.failed.filter((item) => item.id !== file.id)
  }
}
</script>
<template>
  <div
    class="fixed bottom-0 shadow-lg right-0"
    :class="{
      'w-full xl:w-2/5': showTable
    }"
  >
    <div
      class="cursor-pointer overflow-auto dark:text-white"
      @click="showTable = !showTable"
      :class="{
        'bg-redish-50 dark:bg-redish-700': totalItems > 0,
        'bg-brownish-100 dark:bg-brownish-600': totalItems === 0
      }"
    >
      <BaseIcon
        :path="showTable ? mdiChevronDown : mdiChevronUp"
        w="w-6"
        h="h-6"
        class="float-right"
      />
      <span class="text-xs ml-2 mr-2 mt-1 float-right" v-if="totalItems">
        {{ totalItems }}
      </span>
    </div>
    <div class="shadow rounded-sm outline-1">
      <div class="flex gap-2 overflow-auto bg-brownish-50 dark:bg-brownish-800" v-show="showTable">
        <BaseButton
          color="lightDark"
          :label="`All (${totalItems})`"
          :xs="true"
          :outline="true"
          :rounded-full="false"
          @click="tab = undefined"
          :disabled="!tab"
          :active="!tab"
        />
        <BaseButton
          color="lightDark"
          :label="`Active (${activeItems})`"
          :xs="true"
          :outline="true"
          :rounded-full="false"
          @click="tab = 'running'"
          :disabled="tab === 'running'"
          :active="tab === 'running'"
        />
        <BaseButton
          color="lightDark"
          :label="`Pending (${pendingItems})`"
          :xs="true"
          :outline="true"
          :rounded-full="false"
          @click="tab = 'waiting'"
          :disabled="tab === 'waiting'"
          :active="tab === 'waiting'"
        />
        <BaseButton
          color="lightDark"
          :label="`Done (${doneItems})`"
          :xs="true"
          :outline="true"
          :rounded-full="false"
          @click="tab = 'done'"
          :disabled="tab === 'done'"
          :active="tab === 'done'"
        />
        <BaseButton
          color="lightDark"
          :label="`Failed (${failedItems})`"
          :xs="true"
          :outline="true"
          :rounded-full="false"
          @click="tab = 'failed'"
          :active="tab === 'failed'"
        />
      </div>
      <div class="max-h-[325px] overflow-y-scroll bg-brownish-50 dark:bg-brownish-800">
        <div v-show="showTable">
          <template v-for="item in displaying" v-bind:key="item.file.id">
            <SingleFile :file="item.file" :type="item.type" @remove="remove" />
          </template>

          <template v-if="!displaying.length">
            <div class="text-center pb-3 pt-4">No activity in progress</div>
          </template>
        </div>
      </div>
    </div>
  </div>
</template>
