<script setup lang="ts">
import LayoutAuthenticated from '@/layouts/LayoutAuthenticated.vue'
import BreadCrumbs from '@/components/files/BreadCrumbs.vue'
import TableFiles from '@/components/files/TableFiles.vue'
import FileBrowser from '@/components/files/FileBrowser.vue'
import SectionMain from '@/components/ui/SectionMain.vue'
import CardBox from '@/components/ui/CardBox.vue'
import SectionTitleLineWithButton from '@/components/ui/SectionTitleLineWithButton.vue'
import type { DownloadStore, ListAppFile } from '@/stores/storage/types'

const downloadAction = (download: DownloadStore, file: ListAppFile) => {
  return download.push(file)
}
</script>

<template>
  <Suspense>
    <LayoutAuthenticated>
      <FileBrowser v-slot="{ storage, download }">
        <SectionMain>
          <SectionTitleLineWithButton title="" main />

          <CardBox rounded="rounded-md" class="mb-2 px-3 py-1" has-table>
            <BreadCrumbs :parents="storage.parents" />
          </CardBox>

          <CardBox rounded="rounded-md" class="mb-6" has-table>
            <TableFiles
              :items="storage.items"
              :parent="storage.dir || null"
              @download="(file) => downloadAction(download, file)"
            />
          </CardBox>
        </SectionMain>
      </FileBrowser>
    </LayoutAuthenticated>
  </Suspense>
</template>
