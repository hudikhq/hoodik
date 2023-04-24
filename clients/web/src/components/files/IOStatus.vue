<script setup lang="ts">
import { store as uploadStore } from '@/stores/storage/upload'
import { store as downloadStore } from '@/stores/storage/download'
import SingleFile from '../files/SingleFile.vue'
import BaseIcon from '../ui/BaseIcon.vue'
import { computed, ref, watch } from 'vue'
import { mdiChevronDown, mdiChevronUp } from '@mdi/js'

const upload = uploadStore()
const download = downloadStore()

const neverPoppedItUp = ref(true)
const showTable = ref(false)

const hasItems = computed(() => {
  return (
    upload.uploading.length > 0 ||
    upload.waiting.length > 0 ||
    upload.done.length > 0 ||
    upload.failed.length > 0 ||
    download.downloading.length > 0 ||
    download.waiting.length > 0 ||
    download.done.length > 0 ||
    download.failed.length > 0
  )
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
</script>
<template>
  <div class="fixed bottom-0 right-0">
    <div
      class="bg-white dark:bg-gray-800 shadow rounded-sm overflow-hidden outline outline-gray-700"
    >
      <table class="w-full">
        <thead
          @click="showTable = !showTable"
          :class="{
            'cursor-pointer': hasItems
          }"
        >
          <th class="px-4 py-2 text-gray-700 dark:text-white text-sm w-52">Name</th>
          <th class="px-4 py-2 text-gray-700 dark:text-white text-sm w-32">Size</th>
          <th class="px-4 py-2 text-gray-700 dark:text-white text-sm w-32">Status</th>
          <th class="px-4 py-2 text-gray-700 dark:text-white text-sm w-44">
            Speed

            <BaseIcon
              :path="showTable ? mdiChevronDown : mdiChevronUp"
              w="w-6"
              h="h-6"
              class="float-right"
            />
          </th>
        </thead>
        <tbody v-show="showTable">
          <template v-for="file in upload.uploading" v-bind:key="file.id">
            <SingleFile :file="file" type="upload:uploading" />
          </template>

          <template v-for="file in upload.waiting" v-bind:key="file.id">
            <SingleFile :file="file" type="upload:waiting" />
          </template>

          <template v-for="file in upload.done" v-bind:key="file.id">
            <SingleFile :file="file" type="upload:done" />
          </template>

          <template v-for="file in upload.failed" v-bind:key="file.id">
            <SingleFile :file="file" type="upload:failed" />
          </template>

          <template v-for="file in download.downloading" v-bind:key="file.id">
            <SingleFile :file="file" type="download:downloading" />
          </template>

          <template v-for="file in download.waiting" v-bind:key="file.id">
            <SingleFile :file="file" type="download:waiting" />
          </template>

          <template v-for="file in download.done" v-bind:key="file.id">
            <SingleFile :file="file" type="download:done" />
          </template>

          <template v-for="file in download.failed" v-bind:key="file.id">
            <SingleFile :file="file" type="download:failed" />
          </template>

          <tr v-if="!hasItems">
            <td colspan="4" class="text-center text-sm">
              <span>No activity in progress</span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
