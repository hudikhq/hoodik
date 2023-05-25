<script setup lang="ts">
import LayoutAuthenticatedWithLoader from '@/layouts/LayoutAuthenticatedWithLoader.vue'
import TableFiles from '@/components/files/list/TableFiles.vue'
import FileBrowser from '@/components/files/browser/FileBrowser.vue'
import SectionMain from '@/components/ui/SectionMain.vue'
import CardBox from '@/components/ui/CardBox.vue'
import BreadCrumbs from '@/components/files/BreadCrumbs.vue'
import { useRoute, useRouter } from 'vue-router'
import { computed, onMounted, watch } from 'vue'

const route = useRoute()
const router = useRouter()

const parentId = computed(() => {
  if (route.params.file_id) {
    if (Array.isArray(route.params.file_id)) {
      return route.params.file_id[0]
    }

    return route.params.file_id
  }

  return undefined
})

const fileId = computed(() => {
  if (route.query.file) {
    if (Array.isArray(route.query.file)) {
      return route.query.file[0] as string
    }

    return route.query.file as string
  }

  return undefined
})

const scroll = () => {
  if (fileId.value) {
    const element = document.getElementById(`${fileId.value}`)
    const headerOffset = 45
    if (element) {
      const elementPosition = element.getBoundingClientRect().top
      const offsetPosition = elementPosition + window.pageYOffset - headerOffset

      window.scrollTo({
        top: offsetPosition,
        behavior: 'smooth'
      })

      setTimeout(() => {
        router.replace({ query: { ...route.query, file: undefined }, params: route.params })
      }, 5000)
    }
  }
}

onMounted(() => {
  setTimeout(() => {
    scroll()
  }, 250)
})

watch(fileId, () => {
  setTimeout(() => {
    scroll()
  }, 250)
})
</script>

<template>
  <LayoutAuthenticatedWithLoader>
    <FileBrowser
      :parentId="parentId"
      :hide-delete="false"
      :share="true"
      v-slot="{ storage, loading, on }"
    >
      <SectionMain>
        <CardBox rounded="rounded-md" class="mb-2 px-0 py-0" has-table>
          <div class="w-full border-y-0">
            <div class="float-left p-2">
              <BreadCrumbs :parents="storage.parents" :parentId="parentId" />
            </div>
          </div>
        </CardBox>

        <TableFiles
          :searchedFileId="fileId"
          :for-delete="storage.forDelete"
          :parents="storage.parents"
          :items="storage.items"
          :dir="storage.dir || null"
          :hide-checkbox="false"
          :hide-delete="false"
          :show-actions="true"
          :share="true"
          :loading="loading"
          v-on="on"
        />
      </SectionMain>
    </FileBrowser>
  </LayoutAuthenticatedWithLoader>
</template>
