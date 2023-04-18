<script setup lang="ts">
import SectionMain from '@/components/ui/SectionMain.vue'
import TableFiles from '@/components/ui/TableFiles.vue'
import CardBox from '@/components/ui/CardBox.vue'
import LayoutAuthenticated from '@/layouts/LayoutAuthenticated.vue'
import SectionTitleLineWithButton from '@/components/ui/SectionTitleLineWithButton.vue'
import BreadCrumbs from '@/components/ui/BreadCrumbs.vue'
import { store as storageStore } from '@/stores/storage'
import { store as cryptoStore } from '@/stores/crypto'
import { useRoute } from 'vue-router'
import { watch } from 'vue'

const storage = storageStore()
const crypto = cryptoStore()
const route = useRoute()

const load = async () => {
  let file_id = undefined

  if (route.params.file_id !== undefined) {
    file_id = parseInt(route.params.file_id as string)
  }

  await storage.find(crypto.keypair, file_id)
}

watch(
  () => route.params.file_id,
  () => {
    load()
  }
)

load()
</script>

<template>
  <LayoutAuthenticated>
    <SectionMain>
      <SectionTitleLineWithButton title="" main />

      <CardBox rounded="rounded-md" class="mb-2 px-3 py-1" has-table>
        <BreadCrumbs />
      </CardBox>

      <CardBox rounded="rounded-md" class="mb-6" has-table>
        <TableFiles />
      </CardBox>
    </SectionMain>
  </LayoutAuthenticated>
</template>
