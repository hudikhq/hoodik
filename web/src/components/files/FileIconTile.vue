<script setup lang="ts">
import { computed } from 'vue'
import { mdiFileOutline, mdiFolder } from '@mdi/js'
import BaseIcon from '@/components/ui/BaseIcon.vue'

/**
 * Rounded-square stand-in for files without a thumbnail, so every row
 * carries a tile of the same shape whether or not it has an image.
 */
const props = defineProps<{
  mime?: string
  /** Overrides the generic folder/file glyph. */
  path?: string
  /** Size + spacing classes, matching the thumbnail's. */
  tileClass?: string
  /** Glyph colour classes. */
  iconClass?: string
  size?: number
}>()

const iconPath = computed(() => props.path ?? (props.mime === 'dir' ? mdiFolder : mdiFileOutline))
</script>

<template>
  <BaseIcon
    :path="iconPath"
    :size="size ?? 16"
    w=""
    h=""
    class="shrink-0 rounded-md bg-paper-200 dark:bg-brownish-600"
    :class="[
      tileClass ?? 'w-6 h-6',
      iconClass ?? 'text-brownish-200 dark:text-brownish-50'
    ]"
  />
</template>
