<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import type { FilesStore, KeyPair, AppFile } from 'types'
import { notification, humanizeError } from '!/notify'

const props = defineProps<{
  modelValue: AppFile | undefined
  Storage: FilesStore
  kp: KeyPair
}>()

const { t } = useI18n()

const emits = defineEmits<{
  (event: 'update:modelValue', value: AppFile | undefined): void
}>()

const isDirectory = computed(() => props.modelValue?.mime === 'dir')

/**
 * Deleting is permanent — there is no trash to fall back on — so a folder,
 * which takes everything under it, asks for its name to be typed. A single
 * file is one row the reader can see and name in the sentence above, and does
 * not earn the extra friction.
 */
const typed = ref('')

watch(
  () => props.modelValue,
  () => (typed.value = '')
)

const confirmDisabled = computed(
  () => isDirectory.value && typed.value.trim() !== props.modelValue?.name
)

const confirmRemove = async () => {
  if (!props.modelValue || confirmDisabled.value) return

  // The dialog is already closing by the time this runs, so a failure has
  // to announce itself — the row simply staying put reads as success.
  try {
    await props.Storage.remove(props.kp, props.modelValue)
  } catch (err) {
    notification(t('errors.deleteFailed'), humanizeError(err), 'error')
  }

  emits('update:modelValue', undefined)
}
</script>

<template>
  <CardBoxModal
    :title="$t('common.delete')"
    button="danger"
    :model-value="!!props.modelValue"
    :button-label="$t('files.delete.confirmLabel')"
    :has-cancel="true"
    :confirm-disabled="confirmDisabled"
    @cancel="emits('update:modelValue', undefined)"
    @confirm="confirmRemove"
  >
    <p>
      {{
        isDirectory
          ? $t('files.delete.confirmDirectory', { name: props.modelValue?.name })
          : $t('files.delete.confirmFile', { name: props.modelValue?.name })
      }}
    </p>

    <label v-if="isDirectory" class="block mt-4 text-sm">
      <span class="block mb-1.5">
        {{ $t('files.delete.typeName', { name: props.modelValue?.name }) }}
      </span>
      <input
        v-model="typed"
        type="text"
        autocomplete="off"
        data-testid="delete-confirm-name"
        class="w-full bg-white dark:bg-brownish-800 border border-paper-300 dark:border-brownish-700 rounded-lg px-4 py-2 focus:outline-none focus:ring-2 focus:ring-offset-0 focus:ring-redish-400/60 dark:focus:ring-redish-500/50"
      />
    </label>
  </CardBoxModal>
</template>
