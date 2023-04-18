<script setup lang="ts">
import { store as uploadStore } from '@/stores/storage/upload'
import SingleFile from './SingleFile.vue'

const upload = uploadStore()
</script>
<template>
  <div class="fixed bottom-0 right-0 m-4 p-4 bg-white dark:bg-gray-800 rounded-lg shadow-lg">
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
