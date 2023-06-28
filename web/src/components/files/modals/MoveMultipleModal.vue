<script setup lang="ts">
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import type { AppFile, FilesStore, KeyPair } from 'types'
import DirectoryTree from '@/components/files/browser/DirectoryTree.vue'
import { useRouter } from 'vue-router'

const router = useRouter()

const props = defineProps<{
  modelValue: boolean
  Storage: FilesStore
  kp: KeyPair
}>()

const emits = defineEmits<{
  (event: 'update:modelValue', value: boolean): void
}>()

const select = async (file?: AppFile) => {
  if (!props.Storage.selected) {
    return
  }

  await props.Storage.moveAll(props.kp, props.Storage.selected, file?.id)

  emits('update:modelValue', false)

  router.push({ name: 'files', params: { file_id: file?.id } })
}
</script>

<template>
  <CardBoxModal
    title="Select target directory"
    :model-value="props.modelValue"
    :has-cancel="false"
    :has-close="true"
    :hide-submit="true"
    @cancel="emits('update:modelValue', false)"
  >
    <div class="w-full border-[1px] border-t-0 border-brownish-800 min-h-[400px]">
      <DirectoryTree @select="select" :keypair="props.kp" :Storage="props.Storage" load />
    </div>
  </CardBoxModal>
</template>
