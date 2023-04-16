<script setup lang="ts">
import { mdiTableBorder } from '@mdi/js'
import SectionMain from '@/components/ui/SectionMain.vue'
import TableFiles from '@/components/ui/TableFiles.vue'
import CardBox from '@/components/ui/CardBox.vue'
import LayoutAuthenticated from '@/layouts/LayoutAuthenticated.vue'
import SectionTitleLineWithButton from '@/components/ui/SectionTitleLineWithButton.vue'
import CardBoxComponentEmpty from '@/components/ui/CardBoxComponentEmpty.vue'

import { store as storageStore } from '@/stores/storage'
import { store as cryptoStore } from '@/stores/crypto'

const storage = storageStore()
const crypto = cryptoStore()

const load = async () => {
  await storage.find(crypto.keypair)
}

load()
</script>

<template>
  <LayoutAuthenticated>
    <SectionMain>
      <SectionTitleLineWithButton :icon="mdiTableBorder" :title="storage.title" main />

      <CardBox class="mb-6" has-table v-if="!storage.loading">
        <TableFiles />
      </CardBox>

      <CardBox v-else>
        <CardBoxComponentEmpty />
      </CardBox>
    </SectionMain>
  </LayoutAuthenticated>
</template>
