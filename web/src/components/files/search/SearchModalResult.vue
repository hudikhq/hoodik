<script setup lang="ts">
import type { AppFile } from 'types'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import { mdiFolderOutline, mdiFileOutline } from '@mdi/js'
import { computed } from 'vue'
import { formatSize } from '!'

const props = defineProps<{
  file: AppFile
}>()

const emits = defineEmits<{
  (event: 'clicked', file: AppFile): void
}>()

const url = computed(() => {
  if (props.file.mime !== 'dir') {
    if (props.file.file_id) {
      return {
        name: 'files',
        params: {
          file_id: props.file.file_id
        },
        hash: `#${props.file.id}`
      }
    } else {
      return {
        name: 'files',
        hash: `#${props.file.id}`
      }
    }
  }

  return {
    name: 'files',
    params: {
      file_id: props.file.id
    }
  }
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
    class="p-2 flex hover:bg-brownish-200 hover:dark:bg-brownish-800 border-b-[1px] border-brownish-50 dark:border-brownish-700"
    :to="url"
    @click="emits('clicked', props.file)"
  >
    <div class="w-1/12 pt-2">
      <img
        v-if="props.file.thumbnail"
        :src="props.file.thumbnail"
        :alt="name"
        class="w-10 h-10 rounded-md"
      />
      <BaseIcon
        v-else-if="props.file.mime === 'dir'"
        :path="mdiFolderOutline"
        :size="28"
        w="w-8"
        h="h-8"
      />
      <BaseIcon v-else :path="mdiFileOutline" :size="28" w="w-8" h="h-8" />
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
