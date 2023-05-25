<script setup lang="ts">
import FilePreview from '@/components/files/preview/FilePreview.vue'
import { useRoute } from 'vue-router'
import type { FilesStore, KeyPair, ListAppFile } from 'types'
import { ref, watch } from 'vue'

const props = defineProps<{
  kp: KeyPair
  Storage: FilesStore
}>()

const emits = defineEmits<{
  (event: 'update:modelValue', value: ListAppFile | undefined): void
  (event: 'details', file: ListAppFile): void
  (event: 'download', file: ListAppFile): void
  (event: 'remove', file: ListAppFile): void
}>()

const route = useRoute()
const file = ref()

watch(
  () => route.params.file_id,
  async (id: string[] | string) => {
    if (!id) return

    const fileId = Array.isArray(id) ? id[0] : id

    file.value = await props.Storage.metadata(fileId, props.kp)
  },
  { immediate: true }
)
</script>
<template>
  <FilePreview
    v-if="file"
    :kp="props.kp"
    :Storage="Storage"
    v-model="file"
    @details="emits('details', $event)"
    @download="emits('download', $event)"
    @remove="emits('remove', $event)"
  />
</template>
