<script setup lang="ts">
import { store as storageStore } from '!/storage'
import { store as cryptoStore } from '!/crypto'
import { store as linksStore } from '!/links'
import DeleteMultipleModal from '@/components/links/modals/DeleteMultipleModal.vue'
import DeleteModal from '@/components/links/modals/DeleteModal.vue'
import LinkModal from '@/components/links/modals/LinkModal.vue'
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

const singleRemove = ref<AppLink>()
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

/**
 * Opens a modal to confirm removing a single file
 */
const remove = (file: AppLink) => {
  singleRemove.value = file
}

const load = async () => {
  await Links.find(Crypto.keypair)
}

await load()
</script>
<template>
  <DeleteModal v-model="singleRemove" :Storage="Storage" :kp="Crypto.keypair" />
  <LinkModal v-model="linkView" :Storage="Storage" :Links="Links" :kp="Crypto.keypair" />
  <DeleteMultipleModal
    v-model="isModalDeleteMultipleActive"
    :Storage="Storage"
    :kp="Crypto.keypair"
  />

  <slot
    :authenticated="props.authenticated"
    :keypair="props.keypair"
    :loading="Links.loading"
    :Links="Links"
    :Storage="Storage"
    :on="{
      link,
      remove,
      'remove-all': removeAll,
      'select-one': Links.selectOne,
      'select-all': Links.selectAll
    }"
  />
</template>
