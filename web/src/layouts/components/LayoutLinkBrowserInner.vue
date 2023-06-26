<script setup lang="ts">
import DeleteMultipleModal from '@/components/links/modals/DeleteMultipleModal.vue'
import LinkModal from '@/components/modals/LinkModal.vue'
import { store as storageStore } from '!/storage'
import { store as cryptoStore } from '!/crypto'
import { store as linksStore } from '!/links'
import { ref } from 'vue'
import type { Authenticated, KeyPair, AppLink } from 'types'

const props = defineProps<{
  clear?: boolean
  authenticated: Authenticated
  keypair: KeyPair
}>()

const Storage = storageStore()
const Crypto = cryptoStore()
const Links = linksStore()

const isModalDeleteMultipleActive = ref(false)

const linkView = ref<AppLink>()

/**
 * Open a modal with file link, create it if it doesn't exist
 */
const link = (file: AppLink) => {
  linkView.value = file
}

/**
 * Opens a modal to confirm removing multiple files
 */
const removeAll = () => {
  isModalDeleteMultipleActive.value = true
}

const load = async () => {
  await Links.find(Crypto.keypair)

  // Load or re-load the stats for the user so it can be properly displayed
  await Storage.loadStats()
}

load()
</script>
<template>
  <LinkModal v-model="linkView" :Storage="Storage" :Links="Links" :kp="Crypto.keypair" />
  <DeleteMultipleModal v-model="isModalDeleteMultipleActive" :Links="Links" :kp="Crypto.keypair" />

  <slot
    :authenticated="props.authenticated"
    :keypair="props.keypair"
    :loading="Links.loading"
    :Links="Links"
    :Storage="Storage"
    :on="{
      link,
      'remove-all': removeAll,
      'select-one': Links.selectOne,
      'select-all': Links.selectAll,
      'deselect-all': Links.deselectAll
    }"
  />
</template>
