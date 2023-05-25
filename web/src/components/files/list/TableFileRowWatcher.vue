<script setup lang="ts">
import TableFileRow from '@/components/files/list/TableFileRow.vue'
import type { ListAppFile } from 'types'
import scrollMonitor from 'scrollmonitor'
import { ref, onMounted } from 'vue'

const props = defineProps<{
  file: ListAppFile
  checkedRows: Partial<ListAppFile>[]
  hideDelete?: boolean
  share?: boolean
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
  (event: 'details', file: ListAppFile): void
  (event: 'link', file: ListAppFile): void
  (event: 'preview', file: ListAppFile): void
  (event: 'download', file: ListAppFile): void
  (event: 'select-one', value: boolean, file: ListAppFile): void
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
  <Suspense>
    <div
      :class="{
        'border-greeny-100 dark:border-greeny-800 border-2': props.highlighted
      }"
      class="w-full flex p-2 dark:bg-brownish-900 hover:bg-dirty-white hover:dark:bg-brownish-700"
      v-if="!visible"
    >
      Loading...
    </div>
    <TableFileRow
      v-else
      :file="props.file"
      :checked-rows="props.checkedRows"
      :hide-delete="props.hideDelete"
      :share="props.share"
      :hide-checkbox="props.hideCheckbox"
      :sizes="props.sizes"
      :highlighted="props.highlighted"
      @actions="(f: ListAppFile) => emits('actions', f)"
      @remove="(f: ListAppFile) => emits('remove', f)"
      @details="(f: ListAppFile) => emits('details', f)"
      @link="(f: ListAppFile) => emits('link', f)"
      @preview="(f: ListAppFile) => emits('preview', f)"
      @download="(f: ListAppFile) => emits('download', f)"
      @select-one="(v: boolean, f: ListAppFile) => emits('select-one', v, f)"
    />
  </Suspense>
  <div ref="referenceObject" :id="props.file.id"></div>
</template>
