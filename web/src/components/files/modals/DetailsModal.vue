<script setup lang="ts">
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import CardBoxComponentTitle from '@/components/ui/CardBoxComponentTitle.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import { mdiFileOutline, mdiFolderOutline, mdiClose } from '@mdi/js'
import { computed } from 'vue'
import type { KeyPair, AppFile } from 'types'
import { formatPrettyDate, formatSize } from '!/index'
import { AppField } from '@/components/form'

const props = defineProps<{
  modelValue: AppFile | undefined
  kp: KeyPair
}>()

const emits = defineEmits<{
  (event: 'update:modelValue', value: AppFile | undefined): void
}>()

const file = computed({
  get: () => props.modelValue,
  set: (value: AppFile | undefined) => emits('update:modelValue', value)
})

const isDir = computed(() => {
  if (!file.value) return false
  return file.value.mime === 'dir'
})

const size = computed(() => {
  if (!file.value) return '-'
  if (file.value.mime === 'dir') return '-'
  return formatSize(file.value.size)
})

const modified = computed(() => {
  if (!file.value) return ''
  return file.value?.file_modified_at ? formatPrettyDate(file.value?.file_modified_at) : ''
})

const fileFinishedUploadAt = computed(() => {
  if (!file.value) return null
  return file.value?.finished_upload_at ? formatPrettyDate(file.value?.finished_upload_at) : null
})

const percentage = computed(() => {
  if (!file.value) return null
  if (fileFinishedUploadAt.value) return null

  const stored = file.value?.chunks_stored || 0
  const total = file.value?.chunks || 0

  return total && stored ? `${Math.round((stored / total) * 100)}%` : '0%'
})

const cancel = () => {
  emits('update:modelValue', undefined)
}
</script>

<template>
  <CardBoxModal
    v-if="file"
    :model-value="!!file"
    :has-cancel="false"
    :hide-submit="true"
    @cancel="cancel"
  >
    <CardBoxComponentTitle
      :icon="file.mime === 'dir' ? mdiFolderOutline : mdiFileOutline"
      :title="file?.name"
    >
      <BaseButton :icon="mdiClose" color="dark" small rounded-full @click.prevent="cancel" />
    </CardBoxComponentTitle>

    <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
      <div class="flex flex-col w-1/2">Name</div>
      <div class="flex flex-col w-1/2">{{ file.name }}</div>
    </div>
    <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
      <div class="flex flex-col w-1/2">Type</div>
      <div class="flex flex-col w-1/2">{{ file.mime }}</div>
    </div>
    <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
      <div class="flex flex-col w-1/2">Size</div>
      <div class="flex flex-col w-1/2">{{ size }}</div>
    </div>
    <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
      <div class="flex flex-col w-1/2">Modified</div>
      <div class="flex flex-col w-1/2">{{ modified }}</div>
    </div>
    <div v-if="!isDir" class="flex flex-row p-2 border-b-[1px] border-brownish-700">
      <div class="flex flex-col w-1/2">Uploaded</div>
      <div class="flex flex-col w-1/2">{{ fileFinishedUploadAt || percentage }}</div>
    </div>
    <div v-if="!isDir && file.md5" class="flex flex-row p-2 border-b-[1px] border-brownish-700">
      <AppField
        label="MD5"
        name="md5"
        class-add="text-xs"
        wrapper-class="w-full"
        :allow-copy="true"
        v-model="file.md5"
        disabled
      />
    </div>
    <div v-if="!isDir && file.sha1" class="flex flex-row p-2 border-b-[1px] border-brownish-700">
      <AppField
        label="SHA1"
        name="sha1"
        class-add="text-xs"
        wrapper-class="w-full"
        :allow-copy="true"
        v-model="file.sha1"
        disabled
      />
    </div>
    <div v-if="!isDir && file.sha256" class="flex flex-row p-2 border-b-[1px] border-brownish-700">
      <AppField
        label="SHA256"
        name="sha256"
        class-add="text-xs"
        wrapper-class="w-full"
        :allow-copy="true"
        v-model="file.sha256"
        disabled
      />
    </div>
    <div v-if="!isDir && file.blake2b" class="flex flex-row p-2 border-b-[1px] border-brownish-700">
      <AppField
        label="BLAKE2b"
        name="blake2b"
        class-add="text-xs"
        wrapper-class="w-full"
        :allow-copy="true"
        v-model="file.blake2b"
        disabled
      />
    </div>
  </CardBoxModal>
</template>
