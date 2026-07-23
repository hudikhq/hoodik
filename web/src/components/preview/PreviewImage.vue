<script setup lang="ts">
import { mdiPlus, mdiMinus } from '@mdi/js'
import BaseButton from '@/components/ui/BaseButton.vue'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import type { Preview } from '!/preview'
import SpinnerIcon from '../ui/SpinnerIcon.vue'
import { heicToJpegBlob } from '!/heic'

const props = defineProps<{
  modelValue: Preview
}>()
const preview = computed(() => props.modelValue)

const container = ref()
const imageUrl = ref<string>()
const thumbnailUrl = ref<string>()
const loadedBytes = ref(0)
const imageW = ref(0)
const imageH = ref(0)
const scaleW = ref(0)
const scaleH = ref(0)

/**
 * Load the preview as image,
 *  - first, fit the thumbnail to the container so we don't have blank screen while loading...
 *  - then load the full image and fit it to the container.
 * HEIC/HEIF files are converted to JPEG via heic2any for cross-platform support.
 */
const load = async () => {
  imageUrl.value = undefined
  thumbnailUrl.value = undefined
  loadedBytes.value = 0

  // Paint the thumbnail first and don't let a failure to find one hold up
  // the real image — it only ever stands in for it.
  try {
    thumbnailUrl.value = await preview.value.loadThumbnail()
    if (thumbnailUrl.value) {
      await fitUrl(thumbnailUrl.value)
    }
  } catch (e) {
    thumbnailUrl.value = undefined
  }

  let blob = new Blob(
    [await preview.value.load((bytes) => (loadedBytes.value = bytes))],
    { type: preview.value.mime }
  )

  if (preview.value.mime === 'image/heic' || preview.value.mime === 'image/heif') {
    try {
      blob = await heicToJpegBlob(blob, 0.9)
    } catch {
      // fall through — let the browser try to render natively (works on Apple platforms)
    }
  }

  const url = URL.createObjectURL(blob)
  await fitUrl(url)
  imageUrl.value = url
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

const downloadPercent = computed(() => {
  const size = preview.value.size
  if (!size || !loadedBytes.value) return 0

  return Math.min(Math.round((loadedBytes.value / size) * 100), 99)
})

/**
 * Keydown event handler
 */
const imageKeydown = (e: KeyboardEvent) => {
  if (e.key === '+') {
    plus()
  }

  if (e.key === '-') {
    minus()
  }

  if (e.key === ' ') {
    fit()
  }
}

onMounted(() => {
  window.addEventListener('keydown', imageKeydown)
})

onUnmounted(() => {
  window.removeEventListener('keydown', imageKeydown)
})

watch(
  () => props.modelValue,
  () => setTimeout(load, 100),
  { immediate: true }
)
</script>

<template>
  <div ref="container" class="w-full h-full image-container">
    <img
      v-if="imageUrl"
      key="original"
      name="original"
      :src="imageUrl"
      :alt="preview.name"
      :height="scaleH"
      :width="scaleW"
    />
    <img
      v-else-if="thumbnailUrl"
      key="thumbnail"
      name="loading-thumbnail"
      :src="thumbnailUrl"
      :alt="preview.name"
      :height="scaleH"
      :width="scaleW"
    />
    <div v-else class="flex flex-col justify-center items-center gap-2 w-full h-full">
      <SpinnerIcon />
      <span v-if="downloadPercent > 0" class="text-sm text-brownish-100">
        {{ downloadPercent }}%
      </span>
    </div>

    <!-- While the thumbnail stands in, the percent rides on top of it so
         the viewer can tell the real image is still on its way. -->
    <div
      v-if="!imageUrl && downloadPercent > 0 && thumbnailUrl"
      class="absolute bottom-20 left-1/2 -translate-x-1/2 px-3 py-1 rounded-full text-sm
        bg-brownish-800/85 text-brownish-100 border border-brownish-600/60"
    >
      {{ downloadPercent }}%
    </div>
  </div>

  <div class="absolute bottom-0 w-full">
    <div class="flex justify-center space-x-4 p-4 pb-6">
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
</template>
<style scoped lang="css">
.image-container {
  display: grid;
  align-content: center;
  justify-content: center;
  /* auto, not scroll: scroll paints both scrollbar tracks even at fit
     scale, which reads as white borders around the image on the dark
     stage. Scrollbars only belong here once the image is zoomed past
     the viewport, and then they should match the stage. */
  overflow: auto;
  scrollbar-color: #393939 transparent;
}

.image-container::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

.image-container::-webkit-scrollbar-track {
  background: transparent;
}

.image-container::-webkit-scrollbar-thumb {
  background: #393939;
  border-radius: 4px;
}

.image-container::-webkit-scrollbar-corner {
  background: transparent;
}

.image-container img {
  max-width: 1000%;
  max-height: 1000%;
}
</style>
