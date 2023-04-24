<script setup lang="ts">
import { mdiTrashCan, mdiCloudCancel } from '@mdi/js'
import BaseButtons from '@/components/ui/BaseButtons.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import TableCheckboxCell from '@/components/ui/TableCheckboxCell.vue'
import { format as formatDate, formatSize } from '@/stores'
import { computed } from 'vue'
import type { ListAppFile } from '@/stores/storage/types'

const props = defineProps<{
  file: ListAppFile
  checkedRows: Partial<ListAppFile>[]
}>()

const emits = defineEmits<{
  (event: 'cancel', file: ListAppFile): void
  (event: 'remove', file: ListAppFile): void
  (event: 'view', file: ListAppFile): void
  (event: 'checked', value: boolean, file: ListAppFile): void
  (event: 'download', file: ListAppFile): void
}>()

const check = (value: boolean) => {
  emits('checked', value, props.file)
}

const checked = computed(() => {
  return !!props.checkedRows.find((item) => item.id === props.file.id)
})

const fileName = computed(() => {
  const name = props.file.metadata?.name || '...'

  return props.file.mime === 'dir' ? `${name}/` : name
})

const fileSize = computed(() => {
  return props.file.size ? formatSize(props.file.size) : ''
})

const fileCreatedAt = computed(() => {
  return props.file.file_created_at ? formatDate(props.file.file_created_at, 'yyyy-MM-dd') : ''
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
  return props.file.finished_upload_at
    ? formatDate(props.file.finished_upload_at, 'yyyy-MM-dd')
    : ''
})
</script>

<template>
  <tr>
    <TableCheckboxCell type="td" v-model="checked" @update:modelValue="check" />
    <td data-label="Name">
      <router-link :to="`/${file.id}`" v-if="file.mime === 'dir'">
        {{ fileName }}
      </router-link>
      <a href="#" @click="emits('download', file)" v-else>
        {{ fileName }}
      </a>
    </td>
    <td data-label="Size">
      {{ fileSize }}
    </td>
    <td data-label="City">
      {{ props.file.mime || '' }}
    </td>
    <td data-label="Created" class="lg:w-1 whitespace-nowrap">
      <small class="text-gray-500 dark:text-slate-400" :title="props.file.file_created_at">
        {{ fileCreatedAt }}
      </small>
    </td>
    <td data-label="Uploaded" class="lg:w-1 whitespace-nowrap">
      <small
        class="text-gray-500 dark:text-slate-400"
        :title="props.file.file_created_at"
        v-if="!props.file.current && !props.file.parent && props.file.finished_upload_at"
      >
        {{ fileFinishedUploadAt }}
      </small>
      <progress
        class="flex w-2/5 self-center lg:w-full"
        :max="100"
        :value="progressValue"
        v-else-if="props.file.mime !== 'dir'"
      />
    </td>
    <td class="before:hidden lg:w-1 whitespace-nowrap">
      <BaseButtons type="justify-start lg:justify-end" no-wrap>
        <!-- <BaseButton color="info" :icon="mdiEye" small @click="isModalActive = true" /> -->
        <BaseButton
          color="danger"
          :icon="mdiTrashCan"
          small
          @click="emits('remove', file)"
          :disabled="!props.file.id"
          v-if="props.file.finished_upload_at || props.file.mime === 'dir'"
        />
        <BaseButton
          color="warning"
          :icon="mdiCloudCancel"
          small
          @click="emits('cancel', file)"
          :disabled="!props.file.id"
          v-else
        />
      </BaseButtons>
    </td>
  </tr>
</template>
