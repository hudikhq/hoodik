<script setup lang="ts">
import PreviewView from '@/components/preview/PreviewView.vue'
import { useRoute, useRouter } from 'vue-router'
import type { FilesStore, KeyPair, AppFile } from 'types'
import { ref, watch } from 'vue'
import { FilePreview } from '!/preview/file'
import type { Preview } from '!/preview'

const props = defineProps<{
  kp: KeyPair
  Storage: FilesStore
}>()

const emits = defineEmits<{
  (event: 'actions', file: AppFile): void
  (event: 'browse'): void
  (event: 'deselect-all'): void
  (event: 'details', file: AppFile): void
  (event: 'directory'): void
  (event: 'download-many'): void
  (event: 'download', file: AppFile): void
  (event: 'link', file: AppFile): void
  (event: 'move-all'): void
  (event: 'remove-all'): void
  (event: 'remove', file: AppFile): void
  (event: 'rename', file: AppFile): void
  (event: 'select-all', files: AppFile[], fileId: string | null | undefined): void
  (event: 'select-one', select: boolean, file: AppFile): void
  (event: 'set-sort-simple', value: string): void
  (event: 'upload-many', files: FileList, dirId?: string): void
}>()

const router = useRouter()
const route = useRoute()
const file = ref()
const preview = ref()

watch(
  () => route.params.id,
  async (id: string[] | string) => {
    if (!id) return

    preview.value = undefined

    const fileId = Array.isArray(id) ? id[0] : id

    file.value = await props.Storage.metadata(fileId, props.kp)

    props.Storage.deselectAll()
    props.Storage.selectOne(true, file.value)

    const p = new FilePreview(file.value, props.kp)
    await p.loadItems()

    preview.value = p
  },
  { immediate: true }
)

/**
 * Navigate to the next or previous file
 */
const nextOrPrevious = (id: string | undefined) => {
  if (!id) return

  router.push({
    name: 'file-preview',
    params: { id }
  })
}

const cancel = (preview: Preview) => {
  router.push({
    name: 'files',
    params: { file_id: preview.parentId },
    hash: `#${preview.id}`
  })
}
</script>
<template>
  <PreviewView
    v-if="preview"
    v-model="preview"
    :hideDelete="false"
    :hidePreviousAndNext="false"
    @details="emits('details', file)"
    @download="emits('download', file)"
    @remove="emits('remove', file)"
    @previous="nextOrPrevious"
    @next="nextOrPrevious"
    @cancel="cancel"
  />
</template>
