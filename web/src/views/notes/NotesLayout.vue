<script setup lang="ts">
import { ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import type { FilesStore, KeyPair } from 'types'
import { FilePreview } from '!/preview/file'
import type { Preview } from '!/preview'
import NotesEditor from './NotesEditor.vue'
import NotesLanding from './NotesLanding.vue'

const props = defineProps<{
  Storage: FilesStore
  keypair: KeyPair
  loading: boolean
}>()

const route = useRoute()
const preview = ref<Preview>()

const fileId = ref<string | undefined>()

const loadError = ref(false)

async function loadFile(id: string) {
  loadError.value = false
  try {
    const file = await props.Storage.metadata(id, props.keypair)
    const p = new FilePreview(file, props.keypair)
    await p.loadItems()
    preview.value = p
  } catch (err) {
    console.error('Failed to load note:', err)
    preview.value = undefined
    loadError.value = true
  }
}

watch(
  () => route.params.id,
  async (id) => {
    const resolved = Array.isArray(id) ? id[0] : (id as string | undefined)
    fileId.value = resolved

    if (!resolved) {
      preview.value = undefined
      return
    }

    await loadFile(resolved)
  },
  { immediate: true }
)
</script>

<template>
  <div class="h-[calc(100vh-4rem)]">
    <div v-if="loadError" class="flex flex-col items-center justify-center h-full text-brownish-400">
      <p class="text-sm">Failed to load note. It may have been deleted or you don't have access.</p>
      <router-link :to="{ name: 'notes' }" class="mt-3 text-sm text-orangy-400 hover:text-orangy-300 underline">
        Back to notes
      </router-link>
    </div>
    <NotesEditor v-else-if="fileId" :preview="preview" />
    <NotesLanding v-else :keypair="keypair" />
  </div>
</template>
