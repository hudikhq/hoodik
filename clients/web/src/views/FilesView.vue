<script setup lang="ts">
import LayoutAuthenticated from '@/layouts/LayoutAuthenticated.vue'
import BreadCrumbs from '@/components/files/BreadCrumbs.vue'
import TableFiles from '@/components/files/TableFiles.vue'
import FileBrowser from '@/components/files/FileBrowser.vue'
import SectionMain from '@/components/ui/SectionMain.vue'
import CardBox from '@/components/ui/CardBox.vue'
import SectionTitleLineWithButton from '@/components/ui/SectionTitleLineWithButton.vue'
</script>

<template>
  <Suspense>
    <LayoutAuthenticated v-slot="{ keypair }">
      <FileBrowser v-slot="{ storage }">
        <SectionMain>
          <SectionTitleLineWithButton title="" main />

          <CardBox rounded="rounded-md" class="mb-2 px-3 py-1" has-table>
            <BreadCrumbs :parents="storage.parents" />
          </CardBox>

          <CardBox rounded="rounded-md" class="mb-6" has-table>
            <TableFiles
              :items="storage.items"
              :checkable="true"
              @download="(file) => storage.get(keypair, file)"
            />
          </CardBox>
        </SectionMain>
      </FileBrowser>
    </LayoutAuthenticated>
  </Suspense>
</template>
