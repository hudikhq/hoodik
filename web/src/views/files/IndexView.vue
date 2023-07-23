<script setup lang="ts">
import LayoutFileBrowser from '@/layouts/LayoutFileBrowser.vue'
import SectionMain from '@/components/ui/SectionMain.vue'
import BreadCrumbs from '@/components/files/BreadCrumbs.vue'
import TableFiles from './index-view/TableFiles.vue'
import { useRoute, useRouter } from 'vue-router'
import { computed, onMounted, watch } from 'vue'

const route = useRoute()
const router = useRouter()

const parentId = computed(() => {
  if (route.hash) {
    if (Array.isArray(route.params.file_id)) {
      return route.params.file_id[0]
    }

    return route.params.file_id
  }

  return undefined
})

const fileId = computed(() => {
  if (route.hash) {
    return route.hash.replace('#', '')
  }

  return undefined
})

const scroll = () => {
  if (fileId.value) {
    const element = document.getElementById(`${fileId.value}`)
    const headerOffset = 100
    if (element) {
      const elementPosition = element.getBoundingClientRect().top
      const offsetPosition = elementPosition + window.pageYOffset - headerOffset

      window.scrollTo({
        top: offsetPosition,
        behavior: 'smooth'
      })

      setTimeout(() => {
        router.push({ hash: undefined })
      }, 5000)
    }
  }
}

onMounted(() => {
  setTimeout(() => {
    scroll()
  }, 500)
})

watch(fileId, (v) => {
  if (v) {
    setTimeout(() => {
      scroll()
    }, 500)
  }
})
</script>

<template>
  <LayoutFileBrowser
    :clear="false"
    :parentId="parentId"
    :hide-delete="false"
    :share="true"
    v-slot="{ Storage, loading, on }"
  >
    <SectionMain>
      <BreadCrumbs :parents="Storage.parents" :parentId="parentId" />

      <TableFiles
        :searchedFileId="fileId"
        :selected="Storage.selected"
        :parents="Storage.parents"
        :parentId="parentId"
        :items="Storage.items"
        :sortOptions="Storage.sortOptions"
        :dir="Storage.dir || null"
        :hide-checkbox="false"
        :hide-delete="false"
        :show-actions="true"
        :share="true"
        :loading="loading"
        v-on="on"
      />
    </SectionMain>
  </LayoutFileBrowser>
</template>
