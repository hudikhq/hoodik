<script setup lang="ts">
import type { AppFile } from 'types'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import FileThumbnail from '@/components/files/FileThumbnail.vue'
import { mdiFolderOutline, mdiFileOutline } from '@mdi/js'
import { computed } from 'vue'
import { formatSize } from '!'
import { isMarkdownFile } from '!/preview'

const props = defineProps<{
  file: AppFile
}>()

const emits = defineEmits<{
  (event: 'clicked', file: AppFile): void
}>()

const canWrite = computed(() => {
  // Owner rows have no `share_role` set; recipients carry their role
  // so we can route Readers to the preview and skip the editor crash.
  if (props.file.is_owner) return true
  const role = props.file.share_role
  return role === 'editor' || role === 'co-owner'
})

const url = computed(() => {
  if (props.file.mime === 'dir') {
    return { name: 'files', params: { file_id: props.file.id } }
  }

  if (isMarkdownFile(props.file) && canWrite.value) {
    return { name: 'notes', params: { id: props.file.id } }
  }

  return { name: 'file-preview', params: { id: props.file.id } }
})

const name = computed(() => {
  if (props.file.mime !== 'dir') {
    return props.file.name
  }

  return `${props.file.name}/`
})

const fileSize = computed(() => {
  return props.file.size ? formatSize(props.file.size) : ''
})
</script>
<template>
  <router-link
    class="flex items-center gap-3 p-2 rounded-lg transition-colors
      hover:bg-brownish-50 dark:hover:bg-brownish-800"
    :to="url"
    @click="emits('clicked', props.file)"
  >
    <div class="shrink-0 text-brownish-300 dark:text-brownish-100">
      <FileThumbnail :file="props.file" img-class="w-10 h-10 rounded-md">
        <BaseIcon
          v-if="props.file.mime === 'dir'"
          :path="mdiFolderOutline"
          :size="28"
          w="w-10"
          h="h-10"
        />
        <BaseIcon v-else :path="mdiFileOutline" :size="28" w="w-10" h="h-10" />
      </FileThumbnail>
    </div>
    <div class="min-w-0 flex-1 text-left">
      <div class="truncate">{{ name }}</div>
      <div class="truncate text-sm text-brownish-300 dark:text-brownish-100">
        {{ props.file.mime }}
      </div>
    </div>
    <div class="shrink-0 text-sm text-brownish-300 dark:text-brownish-100">
      {{ fileSize || '-' }}
    </div>
  </router-link>
</template>
