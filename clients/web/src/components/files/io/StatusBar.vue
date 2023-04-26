<script setup lang="ts">
import { store as filesStore } from '@/stores/storage'
import { store as queueStore } from '@/stores/storage/queue'
import { store as uploadStore } from '@/stores/storage/upload'
import { store as downloadStore } from '@/stores/storage/download'
import StatusOfSingleFile from '@/components/files/io/StatusOfSingleFile.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import { computed, ref, watch, onBeforeMount } from 'vue'
import { mdiChevronDown, mdiChevronUp } from '@mdi/js'
import type { DownloadAppFile, UploadAppFile, QueueItemActionType } from '@/stores/types'
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

const neverPoppedItUp = ref(true)
const showTable = ref(false)
const tab = ref<'running' | 'done' | 'waiting' | 'failed'>()

const items = computed((): InnerFileList[] => {
  const items = [
    ...upload.uploading.map((item: UploadAppFile) => ({
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

const hasItems = computed(() => {
  return !!items.value.length
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
  () => hasItems.value,
  () => {
    if (neverPoppedItUp.value) {
      neverPoppedItUp.value = false
      showTable.value = true
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
    download.running = download.running.filter((item) => item.id !== file.id)
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
  <div class="fixed bottom-0 right-0 w-full md:w-1/3 shadow-lg">
    <div class="shadow rounded-sm outline-1">
      <div
        class="w-full cursor-pointer overflow-auto bg-brownish-50 dark:bg-brownish-700"
        @click="showTable = !showTable"
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
      <div class="overflow-auto bg-brownish-50 dark:bg-brownish-800" v-show="showTable">
        <BaseButton
          class="float-left mr-2"
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
          class="float-left mr-2"
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
          class="float-left mr-2"
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
          class="float-left mr-2"
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
          class="float-left"
          color="lightDark"
          :label="`Failed (${failedItems})`"
          :xs="true"
          :outline="true"
          :rounded-full="false"
          @click="tab = 'failed'"
          :active="tab === 'failed'"
        />
      </div>
      <div class="w-full max-h-[325px] overflow-y-scroll bg-white dark:bg-brownish-800">
        <table>
          <tbody class="table-auto" v-show="showTable">
            <template v-for="item in displaying" v-bind:key="item.file.id">
              <StatusOfSingleFile :file="item.file" :type="item.type" @remove="remove" />
            </template>

            <tr v-if="!displaying.length">
              <td class="text-center text-sm mobile:table-cell pb-6">No activity in progress</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
</template>
