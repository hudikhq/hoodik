<script setup lang="ts">
import { ref, watch } from 'vue'
import type { AppFile } from 'types'
import { store as storageStore } from '!/storage'

/**
 * Lazily loaded file thumbnail. Listings only advertise `has_thumbnail`;
 * this component fetches the encrypted blob per file, decrypts it with
 * the file key and swaps it in — showing a pulsing placeholder meanwhile.
 * Files without a thumbnail render the fallback slot (or nothing).
 */
const props = defineProps<{
  file: AppFile
  /** Size + spacing classes shared by the image and the placeholder. */
  imgClass?: string
}>()

const Storage = storageStore()
const thumbnail = ref<string | undefined>(props.file.thumbnail)
const loading = ref(false)

watch(
  () => [props.file.id, props.file.thumbnail, props.file.has_thumbnail],
  async () => {
    thumbnail.value = props.file.thumbnail

    if (thumbnail.value || !props.file.has_thumbnail) return

    loading.value = true
    try {
      thumbnail.value = await Storage.loadThumbnail(props.file)
    } catch {
      // A missing thumbnail is cosmetic — keep the fallback slot.
    } finally {
      loading.value = false
    }
  },
  { immediate: true }
)
</script>

<template>
  <img
    v-if="thumbnail"
    name="thumbnail"
    :src="thumbnail"
    :alt="file.name"
    :class="imgClass ?? 'w-6 h-6 rounded-md'"
  />
  <span
    v-else-if="loading"
    name="thumbnail-placeholder"
    class="inline-block animate-pulse bg-brownish-100 dark:bg-brownish-700 rounded-md"
    :class="imgClass ?? 'w-6 h-6'"
  />
  <slot v-else />
</template>
