<script setup lang="ts">
import { computed, ref } from 'vue'
import { store as storageStore } from '@/stores/storage'
import { store as cryptoStore } from '@/stores/crypto'
import { store as uploadStore } from '@/stores/storage/upload'

const upload = uploadStore()
const crypto = cryptoStore()
const storage = storageStore()

const props = defineProps<{
  name: string
  class?: string | { [key: string]: boolean }
}>()

const files = ref<{ files: FileList } | null>(null)
const computedClass = computed(() => {
  if (props.class) {
    return props.class
  }

  return 'relative inline-block cursor-pointer rounded-md text-green-200 py-2 px-4 border border-green-300'
})

const add = async () => {
  if (files.value) {
    for (let i = 0; i < files.value?.files?.length; i++) {
      await upload.push(crypto.keypair, files.value?.files?.[i], storage.dir?.id)
    }

    files.value = null
    await storage.find(crypto.keypair, storage.dir?.id)
  }

  if (!upload.active) {
    await upload.start(storage, crypto.keypair)
  }
}
</script>
<template>
  <label for="file-upload" :class="computedClass">
    <slot></slot>
    <input
      id="file-upload"
      name="file-upload"
      type="file"
      multiple
      class="absolute top-0 left-0 opacity-0"
      style="z-index: -1"
      @change="add"
    />
  </label>
</template>
