<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
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

const { t } = useI18n()

const itemSummary = computed(() => t('files.moveShare.itemCount', props.itemCount))

const memberSummary = computed(() => {
  const labels = props.memberLabels
  if (labels.length === 0) return t('files.moveShare.itsMembers')
  if (labels.length <= 3) return labels.join(', ')
  return t('files.moveShare.andMore', {
    names: labels.slice(0, 3).join(', '),
    count: labels.length - 3
  })
})

const progressPercent = computed(() =>
  props.progress == null ? null : Math.round(props.progress * 100)
)
</script>

<template>
  <CardBoxModal
    :title="$t('files.moveShare.title')"
    button="info"
    :model-value="props.modelValue"
    :button-label="$t('files.moveShare.confirmLabel')"
    :has-cancel="true"
    @update:model-value="emits('update:modelValue', $event)"
    @cancel="emits('cancel')"
    @confirm="emits('confirm')"
  >
    <p data-testid="move-share-confirm-message">
      {{
        $t('files.moveShare.message', {
          folder: folderName,
          destination: destinationName,
          items: itemSummary,
          members: memberSummary
        })
      }}
    </p>

    <div
      v-if="progressPercent !== null"
      class="mt-4"
      data-testid="move-share-confirm-progress"
    >
      <div class="h-2 w-full rounded bg-paper-200 dark:bg-brownish-800">
        <div
          class="h-2 rounded bg-redish-500 transition-all"
          :style="{ width: `${progressPercent}%` }"
        />
      </div>
      <p class="mt-1 text-sm">{{ $t('files.moveShare.preparing', { percent: progressPercent }) }}</p>
    </div>
  </CardBoxModal>
</template>
