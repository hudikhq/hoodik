<script setup lang="ts">
import { mdiTrashCan, mdiEye, mdiDownload, mdiDotsVertical } from '@mdi/js'
import BaseButton from '@/components/ui/BaseButton.vue'
import TableCheckboxCell from '@/components/ui/TableCheckboxCell.vue'
import TruncatedSpan from '@/components/ui/TruncatedSpan.vue'
import { formatPrettyDate, formatSize } from '@/stores'
import type { ListAppFile } from '@/types'
import { computed } from 'vue'

const props = defineProps<{
  file: ListAppFile
  checkedRows: Partial<ListAppFile>[]
  hideDelete?: boolean
  hideCheckbox?: boolean
  highlighted?: boolean
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

const emits = defineEmits<{
  (event: 'actions', file: ListAppFile): void
  (event: 'remove', file: ListAppFile): void
  (event: 'preview', file: ListAppFile): void
  (event: 'download', file: ListAppFile): void
  (event: 'select-one', value: boolean, file: ListAppFile): void
}>()

const selectOne = (value: boolean) => {
  emits('select-one', value, props.file)
}

const checked = computed({
  get: () => !!props.checkedRows.find((item) => item.id === props.file.id),
  set: (v) => selectOne(v)
})

const isDir = computed(() => {
  return props.file.mime === 'dir'
})

const fileName = computed(() => {
  const name = props.file.metadata?.name || '...'

  return props.file.mime === 'dir' ? `${name}/` : name
})

const fileSize = computed(() => {
  return props.file.size ? formatSize(props.file.size) : ''
})

const fileCreatedAt = computed(() => {
  return props.file.file_created_at ? formatPrettyDate(props.file.file_created_at) : ''
})

const progressValue = computed(() => {
  const total = props.file.chunks

  if (!total || props.file.finished_upload_at) {
    return 100
  }

  const uploaded = props.file.chunks_stored || 0
  const progress = uploaded / total
  return progress * 100
})

const fileFinishedUploadAt = computed(() => {
  return props.file.finished_upload_at ? formatPrettyDate(props.file.finished_upload_at) : ''
})

const sharedClass = computed(() => {
  return 'dark:bg-brownish-900 hover:bg-dirty-white hover:dark:bg-brownish-700'
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
  <div
    class="w-full flex"
    :class="{
      'bg-greeny-100 dark:bg-greeny-900 hover:bg-greeny-200 hover:dark:bg-greeny-800':
        props.highlighted,
      [sharedClass]: true
    }"
  >
    <div :class="sizes.name" :title="fileName">
      <div :class="sizes.checkbox">
        <TableCheckboxCell v-if="!props.hideCheckbox" v-model="checked" />
      </div>

      <router-link class="font-bold" :to="`/directory/${file.id}`" v-if="isDir">
        <TruncatedSpan :middle="fileName.length > 50" :text="fileName" />
      </router-link>

      <a class="font-bold" href="#" @click="emits('preview', file)" v-if="file.metadata?.thumbnail">
        <img
          v-if="file.metadata?.thumbnail"
          :src="file.metadata?.thumbnail"
          :alt="fileName"
          class="h-6 mr-2 mb-1 inline-block"
        />
        <div class="inline-block">
          <TruncatedSpan :middle="fileName.length > 50" :text="fileName" />
        </div>
      </a>
      <span v-else>
        <TruncatedSpan :middle="fileName.length > 50" :text="fileName" />
      </span>
    </div>

    <div :class="sizes.size" :title="fileSize">
      <span>{{ fileSize || '-' }}</span>
    </div>

    <div :class="sizes.type" :title="props.file.mime">
      <TruncatedSpan :text="props.file.mime" />
    </div>

    <div :class="sizes.createdAt" :title="props.file.file_created_at">
      <TruncatedSpan :text="fileCreatedAt" />
    </div>

    <div :class="sizes.uploadedAt">
      <TruncatedSpan v-if="props.file.finished_upload_at" :text="fileFinishedUploadAt" />
      <progress
        class="self-center w-full"
        :max="100"
        :value="progressValue"
        v-else-if="props.file.mime !== 'dir'"
      />
    </div>

    <div class="hidden xl:block" :class="sizes.buttons">
      <BaseButton
        v-if="file.metadata?.thumbnail"
        color="lightDark"
        :icon="mdiEye"
        small
        @click="emits('preview', file)"
        :disabled="!props.file.id"
      />
      <BaseButton
        v-else
        color="lightDark"
        :icon="mdiDownload"
        small
        @click="emits('download', file)"
        :disabled="!props.file.id || isDir"
      />
      <BaseButton
        v-if="!hideDelete"
        color="danger"
        :icon="mdiTrashCan"
        small
        class="ml-2"
        @click="emits('remove', file)"
        :disabled="!props.file.id"
      />
    </div>
    <div class="xl:hidden" :class="sizes.buttons">
      <BaseButton
        v-if="!hideDelete"
        color="lightDark"
        :icon="mdiDotsVertical"
        small
        class="ml-2"
        @click="emits('actions', file)"
        :disabled="!props.file.id"
      />
    </div>
  </div>
</template>
