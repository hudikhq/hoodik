<script setup lang="ts">
import { store as uploadStore } from '@/stores/storage/upload'
import { store as cryptoStore } from '@/stores/crypto'
import SingleFile from './SingleFile.vue'

import { ref } from 'vue'

const upload = uploadStore()
const crypto = cryptoStore()

const files = ref<{ files: FileList } | null>(null)

const add = async () => {
  if (files.value) {
    for (let i = 0; i < files.value?.files?.length; i++) {
      await upload.push(crypto.keypair, files.value?.files?.[i])
    }

    files.value = null
  }

  await upload.start()
}
</script>
<template>
  <div class="absolute right-3 bottom-3 px-2 py-2 rounded-lg bg-white dark:bg-gray-600">
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

    <input type="file" ref="files" multiple @change="add" />
  </div>
</template>
