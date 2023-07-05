<script setup lang="ts">
import TableFileRow from './TableFileRow.vue'
import scrollMonitor from 'scrollmonitor'
import type { AppFile } from 'types'
import { ref, onMounted } from 'vue'

const props = defineProps<{
  file: AppFile
  checkedRows: Partial<AppFile>[]
  hideDelete?: boolean
  share?: boolean
  hideCheckbox?: boolean
  highlighted?: boolean
  sizes: {
    checkbox: string
    name: string
    size: string
    type: string
    modifiedAt: string
    buttons: string
  }
}>()

const emits = defineEmits<{
  (event: 'actions', file: AppFile): void
  (event: 'deselect-all'): void
  (event: 'details', file: AppFile): void
  (event: 'download', file: AppFile): void
  (event: 'link', file: AppFile): void
  (event: 'remove', file: AppFile): void
  (event: 'rename', file: AppFile): void
  (event: 'select-one', value: boolean, file: AppFile): void
  (event: 'upload-many', files: FileList, dirId?: string): void
}>()

const referenceObject = ref()
const visible = ref(false)

onMounted(() => {
  const elementWatcher = scrollMonitor.create(referenceObject.value, 2000)
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
      class="w-full flex pl-11 pt-3.5 pb-3.5 dark:bg-brownish-900 hover:bg-dirty-white hover:dark:bg-brownish-700"
      v-if="!visible"
    >
      <div class="flex items-start">
        <div class="w-6 h-6 mr-2 rounded-md bg-brownish-50 dark:bg-brownish-800"></div>
        <div class="w-32 h-6 mr-2 rounded-md bg-brownish-50 dark:bg-brownish-800"></div>
      </div>
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
      @actions="(f: AppFile) => emits('actions', f)"
      @deselect-all="() => emits('deselect-all')"
      @details="(f: AppFile) => emits('details', f)"
      @download="(f: AppFile) => emits('download', f)"
      @link="(f: AppFile) => emits('link', f)"
      @remove="(f: AppFile) => emits('remove', f)"
      @rename="(f: AppFile) => emits('rename', f)"
      @select-one="(v: boolean, f: AppFile) => emits('select-one', v, f)"
      @upload-many="(f: FileList, d?: string) => emits('upload-many', f, d)"
    />
  </Suspense>
  <div ref="referenceObject" :id="props.file.id"></div>
</template>
