<script setup lang="ts">
import FileViewInner from './FileViewInner.vue'
import { useRoute } from 'vue-router'
import type { FilesStore, KeyPair, AppFile } from 'types'
import { ref, watch } from 'vue'

const props = defineProps<{
  kp: KeyPair
  Storage: FilesStore
}>()

const emits = defineEmits<{
  (event: 'update:modelValue', value: AppFile | undefined): void
  (event: 'details', file: AppFile): void
  (event: 'download', file: AppFile): void
  (event: 'remove', file: AppFile): void
}>()

const route = useRoute()
const file = ref()

watch(
  () => route.params.id,
  async (id: string[] | string) => {
    if (!id) return

    const fileId = Array.isArray(id) ? id[0] : id

    file.value = await props.Storage.metadata(fileId, props.kp)
  },
  { immediate: true }
)
</script>
<template>
  <FileViewInner
    v-if="file"
    :kp="props.kp"
    :Storage="Storage"
    v-model="file"
    @details="emits('details', $event)"
    @download="emits('download', $event)"
    @remove="emits('remove', $event)"
  />
</template>
