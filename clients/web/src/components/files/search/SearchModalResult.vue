<script setup lang="ts">
import type { ListAppFile } from '@/types'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import { mdiFolderOutline, mdiFileOutline } from '@mdi/js'
import { computed } from 'vue'
import { formatSize } from '@/stores'

const props = defineProps<{
  file: ListAppFile
}>()

const emits = defineEmits<{
  (event: 'clicked'): void
}>()

const url = computed(() => {
  if (props.file.mime !== 'dir') {
    if (props.file.file_id) {
      return `/directory/${props.file.file_id}?file=${props.file.id}`
    } else {
      return `/directory?file=${props.file.id}`
    }
  }

  return `/directory/${props.file.id}`
})

const name = computed(() => {
  if (props.file.mime !== 'dir') {
    return props.file.metadata?.name
  }

  return `${props.file.metadata?.name}/`
})

const fileSize = computed(() => {
  return props.file.size ? formatSize(props.file.size) : ''
})
</script>
<template>
  <router-link
    class="p-2 flex hover:bg-brownish-200 hover:dark:bg-brownish-800 border-b-[1px] border-brownish-50 dark:border-brownish-700"
    :to="url"
    @click="emits('clicked')"
  >
    <div class="w-1/12 pt-2">
      <BaseIcon v-if="props.file.mime === 'dir'" :path="mdiFolderOutline" :size="30" />
      <BaseIcon v-else :path="mdiFileOutline" :size="30" />
    </div>
    <div class="w-7/12 text-left">
      <div class="w-full text-left truncate">
        {{ name }}
      </div>
      <div class="w-full text-left truncate text-sm">
        {{ props.file.mime }}
      </div>
    </div>
    <div class="w-4/12 text-right truncate text-sm">{{ fileSize || '-' }}</div>
  </router-link>
</template>
