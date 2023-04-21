<script setup lang="ts">
import { store as uploadStore } from '@/stores/storage/upload'
import SingleFile from '../actions/SingleFile.vue'
import { computed } from 'vue'

const upload = uploadStore()

const hide = computed(() => {
  return (
    upload.uploading.length === 0 &&
    upload.waiting.length === 0 &&
    upload.done.length === 0 &&
    upload.failed.length === 0
  )
})
</script>
<template>
  <div
    class="fixed bottom-0 right-0 m-4 p-4 bg-white dark:bg-gray-800 rounded-lg shadow-lg"
    v-show="!hide"
  >
    <template v-for="file in upload.uploading" v-bind:key="file.id">
      <SingleFile :file="file" type="upload" />
    </template>

    <template v-for="file in upload.waiting" v-bind:key="file.id">
      <SingleFile :file="file" type="waiting" />
    </template>

    <template v-for="file in upload.done" v-bind:key="file.id">
      <SingleFile :file="file" type="done" />
    </template>

    <template v-for="file in upload.failed" v-bind:key="file.id">
      <SingleFile :file="file" type="failed" />
    </template>
  </div>
</template>
