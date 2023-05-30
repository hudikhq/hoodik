<script setup lang="ts">
import { mdiDownload, mdiFileOutline, mdiPlus, mdiMinus } from '@mdi/js'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import type { LinksStore, AppLink } from 'types'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { formatPrettyDate, formatSize } from '!/index'

const props = defineProps<{
  modelValue: AppLink
  hideDelete?: boolean
  Links: LinksStore
}>()

const emits = defineEmits<{
  (event: 'update:modelValue', value: AppLink): void
  (event: 'download', link: AppLink): void
}>()

const container = ref()
const imageUrl = ref<string>()
const imageW = ref(0)
const imageH = ref(0)
const scaleW = ref(0)
const scaleH = ref(0)

const link = computed({
  get: () => props.modelValue,
  set: (value: AppLink) => emits('update:modelValue', value)
})

const linkExpiresAt = computed(() => {
  return link.value?.expires_at ? formatPrettyDate(link.value?.expires_at) : null
})

const isExpired = computed(() => {
  const now = new Date()

  return link.value?.expires_at && new Date(link.value?.expires_at) < now
})

const name = computed(() => link.value?.name)
const thumbnail = computed(() => {
  if (!link.value) return
  if (isExpired.value) return

  return link.value?.thumbnail
})
const hasPreview = computed(() => !!thumbnail.value)

/**
 * Load the link data
 */
const get = async () => {
  if (!link.value) return

  if (hasPreview.value) {
    return load()
  }
}

/**
 * Load the binary data of the link from the backend
 */
const load = async () => {
  if (!link.value || !thumbnail.value) return

  await fitUrl(thumbnail.value)

  if (!imageUrl.value) {
    const response = await props.Links.download(link.value.id, link.value.link_key_hex)

    const url = URL.createObjectURL(await response.blob())

    await fitUrl(url)

    imageUrl.value = url
  }
}

/**
 * Start the download of a link through the
 * regular download process.
 */
const download = async () => {
  if (!link.value) return

  await props.Links.formDownload(link.value.id, link.value.link_key_hex)
}

/**
 * Add to the image scale
 */
const plus = () => {
  scaleW.value *= 1.25
  scaleH.value *= 1.25
}

/**
 * Deduct from the image scale
 */
const minus = () => {
  scaleW.value *= 0.75
  scaleH.value *= 0.75
}

/**
 * Fit given URL to the container
 */
const fitUrl = (url: string) =>
  new Promise((resolve) => {
    const img = new Image()
    img.onload = () => {
      imageW.value = img.width
      imageH.value = img.height

      resolve(fit())
    }

    img.src = url
  })

/**
 * Fit the image to display fully within the available space
 */
const fit = () => {
  if (!container.value) return

  const aspectRatio = imageW.value / imageH.value
  const containerWidth = container.value.offsetWidth * 0.75
  const containerHeight = container.value.offsetHeight * 0.75
  const containerAspectRatio = containerWidth / containerHeight

  if (aspectRatio > containerAspectRatio) {
    scaleW.value = containerWidth
    scaleH.value = containerWidth / aspectRatio
  } else {
    scaleH.value = containerHeight
    scaleW.value = containerHeight * aspectRatio
  }
}

/**
 * Percentage of the image scale
 */
const percentage = computed(() => {
  if (imageW.value === 0 || scaleW.value === 0) return -1

  return Math.round((scaleW.value / imageW.value) * 100)
})

watch(
  () => props.modelValue,
  () => setTimeout(get, 100),
  { immediate: true }
)

/**
 * Keydown event handler
 */
const keydown = (e: KeyboardEvent) => {
  if (e.key === '+' && props.modelValue) {
    plus()
  }

  if (e.key === '-' && props.modelValue) {
    minus()
  }
  if (e.key === ' ' && props.modelValue) {
    fit()
  }
}

onMounted(() => {
  window.addEventListener('keydown', keydown)
})

onUnmounted(() => {
  window.removeEventListener('keydown', keydown)
})
</script>

<template>
  <div
    v-if="link"
    class="fixed top-0 left-0 flex flex-col items-center justify-center w-full h-full dark:bg-brownish-950 pt-20 pb-20"
  >
    <div class="absolute bottom-0" v-if="linkExpiresAt">
      <span v-if="!isExpired">This link will expire on {{ linkExpiresAt }}</span>
      <span v-else class="text-redish-300">This link has expired on {{ linkExpiresAt }}</span>
    </div>
    <div class="absolute top-0 w-full">
      <div class="float-right space-x-4 p-4">
        <BaseButton
          color="light"
          :icon="mdiDownload"
          small
          name="preview-download"
          @click="download"
          :disabled="!link || isExpired"
        />
      </div>
      <div class="float-left space-x-4 p-4">
        <h1>{{ link.name }} {{ formatSize(link.file_size) }}</h1>
      </div>
    </div>

    <div ref="container" class="w-[100%] h-[calc(100%+2rem)] image-container">
      <template v-if="hasPreview">
        <img
          key="original"
          name="original"
          v-if="imageUrl"
          :src="imageUrl"
          :alt="link.name"
          :height="scaleH"
          :width="scaleW"
        />
        <img
          key="thumbnail"
          name="loading-thumbnail"
          v-else
          :src="thumbnail"
          :alt="name"
          :height="scaleH"
          :width="scaleW"
        />
      </template>
      <div class="flex flex-col" v-else>
        <div class="mb-4 text-center">
          <BaseIcon :path="mdiFileOutline" :size="75" h="h-75" w="w-75" />
        </div>
        <div class="text-center">
          <span> No preview available 🥲 </span>
        </div>
        <div class="text-center mt-2">
          <BaseButton
            color="light"
            :icon="mdiDownload"
            small
            name="preview-download-big"
            label="Download"
            @click="download"
            :disabled="!link || isExpired"
          />
        </div>
      </div>
    </div>

    <div class="absolute bottom-0 w-full">
      <div class="flex justify-center space-x-4 p-8" v-if="hasPreview">
        <BaseButton
          :disabled="!imageUrl || percentage <= 1"
          color="dark"
          :icon="mdiMinus"
          small
          @click="minus"
          title="Decrease image size (+)"
        />
        <BaseButton
          :disabled="!imageUrl"
          color="dark"
          small
          @click="fit"
          title="Fit image to screen (space)"
          :label="percentage > -1 ? `${percentage}%` : ' '"
        />
        <BaseButton
          :disabled="!imageUrl"
          color="dark"
          :icon="mdiPlus"
          small
          @click="plus"
          title="Increase image size (+)"
        />
      </div>
    </div>
  </div>
</template>
<style scoped lang="css">
.image-container {
  display: grid;
  align-content: center;
  justify-content: center;
  overflow: scroll;
}

.image-container img {
  max-width: 1000%;
  max-height: 1000%;
}
</style>
