<script setup lang="ts">
import { mdiTrashCan } from '@mdi/js'
import BaseButtons from '@/components/ui/BaseButtons.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import TableCheckboxCell from '@/components/ui/TableCheckboxCell.vue'
import { format as formatDate } from '@/stores'
import { format as formatSize, type ListAppFile } from '@/stores/storage'
import { computed } from 'vue'

const props = defineProps<{
  file: ListAppFile
  checkable?: boolean | undefined
}>()

const emits = defineEmits<{
  (event: 'remove', file: ListAppFile): void
  (event: 'view', file: ListAppFile): void
  (event: 'checked', file: ListAppFile): void
  (event: 'download', file: ListAppFile): void
}>()

const removeFIle = (file: ListAppFile) => {
  emits('remove', file)
}

const fileName = computed(() => {
  return props.file.metadata?.name || '...'
})
</script>

<template>
  <tr>
    <TableCheckboxCell type="td" v-if="checkable" @checked="$emit('checked', file)" />
    <td data-label="Name">
      <router-link :to="`/${file.id}`" v-if="file.mime === 'dir'">
        {{ fileName }}
      </router-link>
      <a href="#" @click="emits('download', file)" v-else>
        {{ fileName }}
      </a>
    </td>
    <td data-label="Size">
      {{ props.file.size ? formatSize(props.file.size) : '' }}
    </td>
    <td data-label="City">
      {{ props.file.mime || '' }}
    </td>
    <td data-label="Created" class="lg:w-1 whitespace-nowrap">
      <small class="text-gray-500 dark:text-slate-400" :title="props.file.file_created_at">
        {{ props.file.file_created_at ? formatDate(props.file.file_created_at, 'yyyy-MM-dd') : '' }}
      </small>
    </td>
    <td data-label="Uploaded" class="lg:w-1 whitespace-nowrap">
      <small
        class="text-gray-500 dark:text-slate-400"
        :title="props.file.file_created_at"
        v-if="!props.file.current && !props.file.parent && props.file.finished_upload_at"
      >
        {{ formatDate(props.file.finished_upload_at, 'yyyy-MM-dd') }}
      </small>
      <progress
        class="flex w-2/5 self-center lg:w-full"
        max="100"
        :value="(props.file.chunks_stored || 0) / (props.file.chunks || 1)"
        v-else-if="props.file.mime !== 'dir'"
      >
        {{ `${props.file.chunks_stored} / ${props.file.chunks}` }}
      </progress>
    </td>
    <td class="before:hidden lg:w-1 whitespace-nowrap">
      <BaseButtons type="justify-start lg:justify-end" no-wrap>
        <!-- <BaseButton color="info" :icon="mdiEye" small @click="isModalActive = true" /> -->
        <BaseButton
          color="danger"
          :icon="mdiTrashCan"
          small
          @click="() => removeFIle(file)"
          :disabled="!props.file.id"
        />
      </BaseButtons>
    </td>
  </tr>
</template>
