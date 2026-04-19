<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { mdiFileDocumentOutline, mdiPlus, mdiMagnify } from '@mdi/js'
import { formatSize } from '!'
import { search } from '!/storage'
import * as meta from '!/storage/meta'
import { isMarkdownFile } from '!/preview'
import type { KeyPair, AppFile } from 'types'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import CreateNoteModal from './CreateNoteModal.vue'

const props = defineProps<{
  keypair: KeyPair
}>()

const router = useRouter()
const notes = ref<AppFile[]>([])
const loading = ref(true)
const query = ref('')
const showCreateModal = ref(false)
let debounceTimer: ReturnType<typeof setTimeout> | null = null

async function loadRecent() {
  loading.value = true

  const response = await meta.find({
    editable: true,
    order: 'desc',
    order_by: 'modified_at'
  })

  const privateKey = props.keypair.input as string

  const items = await Promise.all(
    response.children.map(async (item) => {
      const decrypted = await meta.decrypt(item, privateKey)
      return { ...item, ...decrypted } as AppFile
    })
  )

  notes.value = items.filter((f) => f.mime !== 'dir' && isMarkdownFile(f))
  loading.value = false
}

async function searchNotes(q: string) {
  loading.value = true

  try {
    const results = await search(q, props.keypair, { editable: true, limit: 50 })
    notes.value = results.filter((f) => f.mime !== 'dir' && isMarkdownFile(f))
  } catch {
    notes.value = []
  }

  loading.value = false
}

watch(query, (q) => {
  if (debounceTimer) clearTimeout(debounceTimer)

  if (!q.trim()) {
    loadRecent()
    return
  }

  debounceTimer = setTimeout(() => searchNotes(q.trim()), 300)
})

function openNote(file: AppFile) {
  router.push({ name: 'notes', params: { id: file.id } })
}

function formatDate(timestamp: number): string {
  const date = new Date(timestamp * 1000)
  const now = new Date()
  const diff = now.getTime() - date.getTime()
  const days = Math.floor(diff / (1000 * 60 * 60 * 24))

  if (days === 0) return 'Today'
  if (days === 1) return 'Yesterday'
  if (days < 7) return `${days} days ago`

  return date.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' })
}

function onNoteCreated() {
  loadRecent()
}

onMounted(() => loadRecent())

onUnmounted(() => {
  if (debounceTimer) clearTimeout(debounceTimer)
})
</script>

<template>
  <div class="h-full flex flex-col p-6 max-w-4xl mx-auto">
    <div class="flex items-center justify-between mb-6">
      <h1 class="text-xl font-semibold dark:text-brownish-100">Notes</h1>
      <BaseButton
        :icon="mdiPlus"
        label="New Note"
        color="info"
        small
        @click="showCreateModal = true"
      />
    </div>

    <div class="relative mb-4">
      <BaseIcon
        :path="mdiMagnify"
        :size="18"
        class="absolute left-3 top-1/2 -translate-y-1/2 text-brownish-400"
      />
      <input
        v-model="query"
        type="text"
        placeholder="Search notes..."
        class="w-full pl-10 pr-4 py-2 text-sm rounded-lg border border-brownish-200 dark:border-brownish-700 bg-white dark:bg-brownish-800 dark:text-brownish-100 focus:outline-none focus:ring-1 focus:ring-orangy-500 focus:border-orangy-500"
      />
    </div>

    <div v-if="loading" class="flex-1 flex items-center justify-center text-brownish-400">
      <p class="text-sm">Loading notes...</p>
    </div>

    <div v-else-if="!notes.length && !query" class="flex-1 flex flex-col items-center justify-center text-brownish-400">
      <BaseIcon :path="mdiFileDocumentOutline" :size="64" />
      <p class="mt-4 text-sm">No notes yet</p>
      <BaseButton
        :icon="mdiPlus"
        label="Create your first note"
        color="info"
        small
        class="mt-4"
        @click="showCreateModal = true"
      />
    </div>

    <div v-else-if="!notes.length && query" class="flex-1 flex items-center justify-center text-brownish-400">
      <p class="text-sm">No notes matching "{{ query }}"</p>
    </div>

    <ul v-else class="flex-1 overflow-y-auto space-y-1">
      <li
        v-for="note in notes"
        :key="note.id"
        :title="note.name"
        class="flex items-center gap-3 px-4 py-3 rounded-lg cursor-pointer transition-colors duration-150 hover:bg-brownish-100 dark:hover:bg-brownish-700/50"
        @click="openNote(note)"
      >
        <BaseIcon :path="mdiFileDocumentOutline" :size="20" class="flex-shrink-0 text-orangy-400" />
        <div class="flex-1 min-w-0">
          <p class="text-sm font-medium truncate dark:text-brownish-100">{{ note.name }}</p>
          <p class="text-xs text-brownish-400 mt-0.5">
            {{ formatDate(note.file_modified_at) }}
            <span v-if="note.size" class="ml-2">{{ formatSize(note.size) }}</span>
          </p>
        </div>
      </li>
    </ul>

    <CreateNoteModal
      v-model="showCreateModal"
      :keypair="keypair"
      @created="onNoteCreated"
    />
  </div>
</template>
