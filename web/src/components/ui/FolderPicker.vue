<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { mdiFolder, mdiArrowLeft } from '@mdi/js'
import * as meta from '!/storage/meta'
import type { KeyPair, AppFile } from 'types'
import BaseIcon from '@/components/ui/BaseIcon.vue'

const props = defineProps<{
  keypair: KeyPair
  startId?: string
  startName?: string
}>()

const emit = defineEmits<{
  (e: 'navigate', payload: { id?: string; name: string }): void
}>()

const folders = ref<AppFile[]>([])
const loading = ref(false)
const currentId = ref<string | undefined>(props.startId)
const currentName = ref(props.startName || 'Root')
const stack = ref<{ id?: string; name: string }[]>([])

async function loadFolders(dirId?: string) {
  loading.value = true
  const response = await meta.find({ dir_id: dirId, dirs_only: true })
  const privateKey = props.keypair.input as string

  const items: AppFile[] = []
  for (const item of response.children) {
    const decrypted = await meta.decrypt(item, privateKey)
    items.push({ ...item, ...decrypted } as AppFile)
  }

  items.sort((a, b) => (a.name || '').localeCompare(b.name || ''))
  folders.value = items
  loading.value = false
}

function open(folder: AppFile) {
  stack.value.push({ id: currentId.value, name: currentName.value })
  currentId.value = folder.id
  currentName.value = folder.name
  emit('navigate', { id: folder.id, name: folder.name })
  loadFolders(folder.id)
}

function back() {
  const parent = stack.value.pop()
  if (!parent) return
  currentId.value = parent.id
  currentName.value = parent.name
  emit('navigate', { id: parent.id, name: parent.name })
  loadFolders(parent.id)
}

defineExpose({ currentId, currentName })

onMounted(() => loadFolders(currentId.value))
</script>

<template>
  <div class="border border-brownish-200 dark:border-brownish-700 rounded-lg overflow-hidden">
    <div
      class="flex items-center gap-2 px-3 py-2 bg-brownish-50 dark:bg-brownish-800 border-b border-brownish-200 dark:border-brownish-700 text-sm"
    >
      <button
        v-if="stack.length"
        type="button"
        class="flex items-center gap-1 text-orangy-500 hover:text-orangy-400 transition-colors"
        @click="back"
      >
        <BaseIcon :path="mdiArrowLeft" :size="14" />
      </button>
      <BaseIcon :path="mdiFolder" :size="16" class="text-orangy-400" />
      <span class="text-brownish-600 dark:text-brownish-300 truncate">{{ currentName }}</span>
    </div>

    <div v-if="loading" class="px-3 py-4 text-center text-sm text-brownish-400">Loading...</div>

    <ul v-else-if="folders.length" class="max-h-40 overflow-y-auto">
      <li
        v-for="folder in folders"
        :key="folder.id"
        class="flex items-center gap-2 px-3 py-2 text-sm cursor-pointer transition-colors hover:bg-brownish-50 dark:hover:bg-brownish-700/50"
        @click="open(folder)"
      >
        <BaseIcon :path="mdiFolder" :size="16" class="flex-shrink-0 text-orangy-400" />
        <span class="truncate dark:text-brownish-300">{{ folder.name }}</span>
      </li>
    </ul>

    <div v-else class="px-3 py-3 text-center text-xs text-brownish-400">No subfolders</div>
  </div>
</template>
