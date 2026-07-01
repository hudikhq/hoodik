<script setup lang="ts">
import { ref } from 'vue'

import DeleteMultipleModal from '@/components/links/modals/DeleteMultipleModal.vue'
import LinkModal from '@/components/links/modals/LinkModal.vue'
import TableLinks from '@/views/links/index-view/TableLinks.vue'

import { store as linksStore } from '!/links'
import { store as cryptoStore } from '!/crypto'
import { store as storageStore } from '!/storage'

import type { AppLink } from 'types'

const Links = linksStore()
const Crypto = cryptoStore()
const Storage = storageStore()

const isModalDeleteMultipleActive = ref(false)
const linkView = ref<AppLink>()

async function load(): Promise<void> {
  await Links.find(Crypto.keypair)
  await Storage.loadStats()
}

void load()

function link(file: AppLink): void {
  linkView.value = file
}

function removeAll(): void {
  isModalDeleteMultipleActive.value = true
}
</script>

<template>
  <LinkModal v-model="linkView" :Storage="Storage" :Links="Links" :kp="Crypto.keypair" />
  <DeleteMultipleModal v-model="isModalDeleteMultipleActive" :Links="Links" :kp="Crypto.keypair" />

  <div data-testid="share-hub-public-table">
    <TableLinks
      :items="Links.items"
      :selected="Links.selected"
      @link="link"
      @remove-all="removeAll"
      @select-one="Links.selectOne"
      @select-all="Links.selectAll"
      @deselect-all="Links.deselectAll"
    />
  </div>
</template>
