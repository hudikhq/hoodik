<script setup lang="ts">
import { computed } from 'vue'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'

const props = defineProps<{
  modelValue: boolean
  folderName: string
  destinationName: string
  itemCount: number
  memberLabels: string[]
  /** Re-wrap progress while a large subtree is being prepared, 0..1. */
  progress?: number | null
}>()

const emits = defineEmits<{
  (event: 'update:modelValue', value: boolean): void
  (event: 'confirm'): void
  (event: 'cancel'): void
}>()

const itemSummary = computed(() =>
  props.itemCount === 1 ? '1 item' : `${props.itemCount} items`
)

const memberSummary = computed(() => {
  const labels = props.memberLabels
  if (labels.length === 0) return 'its members'
  if (labels.length <= 3) return labels.join(', ')
  return `${labels.slice(0, 3).join(', ')} and ${labels.length - 3} more`
})

const progressPercent = computed(() =>
  props.progress == null ? null : Math.round(props.progress * 100)
)
</script>

<template>
  <CardBoxModal
    title="Move and share folder?"
    button="info"
    :model-value="props.modelValue"
    button-label="Move and share"
    :has-cancel="true"
    @update:model-value="emits('update:modelValue', $event)"
    @cancel="emits('cancel')"
    @confirm="emits('confirm')"
  >
    <p data-testid="move-share-confirm-message">
      Moving '{{ folderName }}' into '{{ destinationName }}' will share it and its
      {{ itemSummary }} with {{ memberSummary }}.
    </p>

    <div
      v-if="progressPercent !== null"
      class="mt-4"
      data-testid="move-share-confirm-progress"
    >
      <div class="h-2 w-full rounded bg-brownish-800">
        <div
          class="h-2 rounded bg-info-500 transition-all"
          :style="{ width: `${progressPercent}%` }"
        />
      </div>
      <p class="mt-1 text-sm">Preparing {{ progressPercent }}%</p>
    </div>
  </CardBoxModal>
</template>
