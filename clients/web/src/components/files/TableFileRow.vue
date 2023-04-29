<script setup lang="ts">
import { mdiTrashCan, mdiEye, mdiDownload, mdiDotsVertical } from '@mdi/js'
import BaseButton from '@/components/ui/BaseButton.vue'
import TableCheckboxCell from '@/components/ui/TableCheckboxCell.vue'
import TruncatedSpan from '../ui/TruncatedSpan.vue'
import { formatPrettyDate, formatSize } from '@/stores'
import type { ListAppFile } from '@/types'
import { computed, ref } from 'vue'
import type { Helper } from '@/stores/storage/helper'

const props = defineProps<{
  helper: Helper
  file: ListAppFile
  checkedRows: Partial<ListAppFile>[]
  hideDelete?: boolean
  hideCheckbox?: boolean
  sizes: {
    checkbox: string
    name: string
    size: string
    type: string
    createdAt: string
    uploadedAt: string
    buttons: string
  }
}>()

const decrypted = ref(props.file)
const decrypt = async () => {
  decrypted.value = await props.helper.decrypt(props.file)
}

await decrypt()

const emits = defineEmits<{
  (event: 'remove', file: ListAppFile): void
  (event: 'view', file: ListAppFile): void
  (event: 'checked', value: boolean, file: ListAppFile): void
  (event: 'download', file: ListAppFile): void
}>()

const check = (value: boolean) => {
  emits('checked', value, decrypted.value)
}

const checked = computed({
  get: () => !!props.checkedRows.find((item) => item.id === decrypted.value.id),
  set: (v) => check(v)
})

const isDir = computed(() => {
  return decrypted.value.mime === 'dir'
})

const fileName = computed(() => {
  const name = decrypted.value.metadata?.name || '...'

  return decrypted.value.mime === 'dir' ? `${name}/` : name
})

const fileSize = computed(() => {
  return decrypted.value.size ? formatSize(decrypted.value.size) : ''
})

const fileCreatedAt = computed(() => {
  return decrypted.value.file_created_at ? formatPrettyDate(decrypted.value.file_created_at) : ''
})

const progressValue = computed(() => {
  const total = decrypted.value.chunks

  if (!total || decrypted.value.finished_upload_at) {
    return 100
  }

  const uploaded = decrypted.value.chunks_stored || 0
  const progress = uploaded / total
  return progress * 100
})

const fileFinishedUploadAt = computed(() => {
  return decrypted.value.finished_upload_at
    ? formatPrettyDate(decrypted.value.finished_upload_at)
    : ''
})

const sharedClass = computed(() => {
  return 'bg-brownish-100 dark:bg-brownish-900 hover:bg-brownish-200 hover:dark:bg-brownish-700'
})

const border = 'sm:border-l-2 sm:border-brownish-50 sm:dark:border-brownish-950'
const sizes = computed(() => {
  return {
    checkbox: `${props.sizes.checkbox}`,
    name: `${props.sizes.name}`,
    size: `${border} ${props.sizes.size}`,
    type: `${border} ${props.sizes.type}`,
    createdAt: `${border} ${props.sizes.createdAt}`,
    uploadedAt: `${border} ${props.sizes.uploadedAt}`,
    buttons: `${border} ${props.sizes.buttons} text-right`
  }
})
</script>

<template>
  <div class="w-full flex" :class="sharedClass">
    <div :class="sizes.name" :title="fileName">
      <div :class="sizes.checkbox">
        <TableCheckboxCell
          v-if="!props.hideCheckbox"
          v-model="checked"
          @update:modelValue="check"
        />
      </div>

      <router-link class="font-bold" :to="`/directory/${file.id}`" v-if="isDir">
        <TruncatedSpan :middle="fileName.length > 50" :text="fileName" />
      </router-link>
      <span v-else>
        <TruncatedSpan :middle="fileName.length > 50" :text="fileName" />
      </span>
    </div>

    <div :class="sizes.size" :title="fileSize">
      <span>{{ fileSize || '-' }}</span>
    </div>

    <div :class="sizes.type" :title="decrypted.mime">
      <TruncatedSpan :text="decrypted.mime" />
    </div>

    <div :class="sizes.createdAt" :title="decrypted.file_created_at">
      <TruncatedSpan :text="fileCreatedAt" />
    </div>

    <div :class="sizes.uploadedAt">
      <TruncatedSpan
        v-if="!decrypted.current && !decrypted.parent && decrypted.finished_upload_at"
        :text="fileFinishedUploadAt"
      />
      <progress
        class="self-center w-full"
        :max="100"
        :value="progressValue"
        v-else-if="decrypted.mime !== 'dir'"
      />
    </div>

    <div class="hidden xl:block" :class="sizes.buttons">
      <BaseButton
        v-if="file.mime === 'something-image-mime-type-TODO'"
        color="lightDark"
        :icon="mdiEye"
        small
        @click="emits('view', file)"
        :disabled="!decrypted.id"
      />
      <BaseButton
        v-else
        color="lightDark"
        :icon="mdiDownload"
        small
        @click="emits('download', file)"
        :disabled="!decrypted.id || isDir"
      />
      <BaseButton
        v-if="!hideDelete"
        color="danger"
        :icon="mdiTrashCan"
        small
        class="ml-2"
        @click="emits('remove', file)"
        :disabled="!decrypted.id"
      />
    </div>
    <div class="xl:hidden" :class="sizes.buttons">
      <BaseButton
        v-if="!hideDelete"
        color="lightDark"
        :icon="mdiDotsVertical"
        small
        class="ml-2"
        @click="emits('view', file)"
        :disabled="!decrypted.id"
      />
    </div>
  </div>
</template>
