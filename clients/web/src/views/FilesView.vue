<script setup lang="ts">
import LayoutAuthenticated from '@/layouts/LayoutAuthenticated.vue'
import TableFiles from '@/components/files/TableFiles.vue'
import FileBrowser from '@/components/files/FileBrowser.vue'
import SectionMain from '@/components/ui/SectionMain.vue'
import CardBox from '@/components/ui/CardBox.vue'
import BreadCrumbs from '@/components/files/BreadCrumbs.vue'
import { useRoute } from 'vue-router'
import { computed } from 'vue'

const route = useRoute()

const parentId = computed(() => {
  if (route.params.file_id) {
    return parseInt(route.params.file_id as string)
  }

  return undefined
})
</script>

<template>
  <Suspense>
    <LayoutAuthenticated>
      <FileBrowser :parentId="parentId" v-slot="{ storage, loading, on }">
        <SectionMain>
          <CardBox rounded="rounded-md" class="mb-2 px-0 py-0 mt-4" has-table>
            <div class="w-full border-y-0">
              <div class="float-left p-2">
                <BreadCrumbs :parents="storage.parents" :parentId="parentId" />
              </div>
            </div>
          </CardBox>

          <TableFiles
            :for-delete="storage.forDelete"
            :parents="storage.parents"
            :items="storage.items"
            :dir="storage.dir || null"
            :hide-checkbox="false"
            :hide-delete="false"
            :show-actions="true"
            :loading="loading"
            v-on="on"
          />
        </SectionMain>
      </FileBrowser>
    </LayoutAuthenticated>
  </Suspense>
</template>
