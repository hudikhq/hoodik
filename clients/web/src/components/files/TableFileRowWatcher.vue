<script setup lang="ts">
import TableFileRow from '@/components/files/TableFileRow.vue'
import { uuidv4 } from '@/stores'
import type { Helper } from '@/stores/storage/helper'
import type { ListAppFile } from '@/types'
import scrollMonitor from 'scrollmonitor'
import { ref, onMounted } from 'vue'

const id = uuidv4()
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

const emits = defineEmits<{
  (event: 'remove', file: ListAppFile): void
  (event: 'view', file: ListAppFile): void
  (event: 'checked', value: boolean, file: ListAppFile): void
  (event: 'download', file: ListAppFile): void
}>()

const referenceObject = ref()
const visible = ref(false)

onMounted(() => {
  const elementWatcher = scrollMonitor.create(referenceObject.value, 200)
  elementWatcher.enterViewport(() => {
    visible.value = true
  }, false)

  elementWatcher.exitViewport(() => {
    visible.value = false
  }, false)
})
</script>

<template>
  <div ref="referenceObject" :id="id"></div>
  <Suspense>
    <div
      class="w-full flex p-2 bg-brownish-100 dark:bg-brownish-900 hover:bg-brownish-200 hover:dark:bg-brownish-700"
      v-if="!visible"
    >
      Loading...
    </div>
    <TableFileRow
      v-else
      :helper="props.helper"
      :file="props.file"
      :checked-rows="props.checkedRows"
      :hide-delete="props.hideDelete"
      :hide-checkbox="props.hideCheckbox"
      :sizes="props.sizes"
      @remove="(f) => emits('remove', f)"
      @view="(f) => emits('view', f)"
      @checked="(v, f) => emits('checked', v, f)"
      @download="(f) => emits('download', f)"
    />
  </Suspense>
</template>
