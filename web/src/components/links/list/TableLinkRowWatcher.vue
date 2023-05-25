<script setup lang="ts">
import TableLinkRow from '@/components/links/list/TableLinkRow.vue'
import type { AppLink } from 'types'
import scrollMonitor from 'scrollmonitor'
import { ref, onMounted } from 'vue'

const props = defineProps<{
  link: AppLink
  checkedRows: Partial<AppLink>[]
  hideDelete?: boolean
  share?: boolean
  hideCheckbox?: boolean
  highlighted?: boolean
  sizes: {
    checkbox: string
    name: string
    size: string
    createdAt: string
    fileCreatedAt: string
    expiresAt: string
    buttons: string
  }
}>()

const emits = defineEmits<{
  (event: 'link', link: AppLink): void
  (event: 'remove', link: AppLink): void
  (event: 'select-one', value: boolean, link: AppLink): void
}>()

const referenceObject = ref()
const visible = ref(false)

onMounted(() => {
  const elementWatcher = scrollMonitor.create(referenceObject.value, 100)
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
    <TableLinkRow
      v-else
      :file="props.link"
      :checked-rows="props.checkedRows"
      :sizes="props.sizes"
      :highlighted="props.highlighted"
      @link="(f: AppLink) => emits('link', f)"
      @remove="(f: AppLink) => emits('remove', f)"
      @select-one="(v: boolean, f: AppLink) => emits('select-one', v, f)"
    />
  </Suspense>
  <div ref="referenceObject" :id="props.link.id"></div>
</template>
