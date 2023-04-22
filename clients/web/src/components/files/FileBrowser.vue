<script setup lang="ts">
import { store as storageStore } from '@/stores/storage'
import { store as cryptoStore } from '@/stores/crypto'
import { useRoute } from 'vue-router'
import { watch } from 'vue'

const storage = storageStore()
const crypto = cryptoStore()
const route = useRoute()

const load = async () => {
  let file_id = null

  if (route.params.file_id !== undefined) {
    file_id = parseInt(route.params.file_id as string)
  }

  await storage.find(crypto.keypair, file_id)
}

watch(
  () => route.params.file_id,
  () => load()
)

await load()
</script>
<template>
  <slot :storage="storage"></slot>
</template>
