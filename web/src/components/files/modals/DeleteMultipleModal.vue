<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import type { FilesStore, KeyPair } from 'types'
import { notification, humanizeError } from '!/notify'

const props = defineProps<{
  modelValue: boolean
  Storage: FilesStore
  kp: KeyPair
}>()

const { t } = useI18n()

const emits = defineEmits<{
  (event: 'update:modelValue', value: boolean): void
}>()

const selected = computed(() => props.Storage.selected ?? [])
const count = computed(() => selected.value.length)
const folders = computed(() => selected.value.filter((f) => f.mime === 'dir').length)

/**
 * The danger in a bulk delete is not knowing what is in the selection, so the
 * gate is the count itself — typing it means the number was read. A single
 * file keeps the plain confirm; it is one named row.
 */
const typed = ref('')

watch(
  () => props.modelValue,
  () => (typed.value = '')
)

const needsTyping = computed(() => count.value > 1)
const confirmDisabled = computed(() => needsTyping.value && typed.value.trim() !== String(count.value))

const confirmRemoveAll = async () => {
  if (confirmDisabled.value) return
  try {
    await props.Storage.removeAll(props.kp, props.Storage.selected)
  } catch (err) {
    notification(t('errors.deleteFailed'), humanizeError(err), 'error')
  }
  emits('update:modelValue', false)
}
</script>

<template>
  <CardBoxModal
    :title="$t('files.delete.selectedTitle')"
    button="danger"
    :model-value="props.modelValue"
    :button-label="$t('files.delete.confirmLabel')"
    :has-cancel="true"
    :confirm-disabled="confirmDisabled"
    @cancel="emits('update:modelValue', false)"
    @confirm="confirmRemoveAll"
  >
    <template v-if="needsTyping">
      <p>{{ $t('files.delete.confirmMany', { count }) }}</p>
      <p v-if="folders" class="mt-1 text-sm text-brownish-400 dark:text-brownish-50">
        {{ $t('files.delete.confirmManyFolders', { count: folders }) }}
      </p>

      <label class="block mt-4 text-sm">
        <span class="block mb-1.5">{{ $t('files.delete.typeCount', { count }) }}</span>
        <input
          v-model="typed"
          type="text"
          inputmode="numeric"
          autocomplete="off"
          data-testid="delete-confirm-count"
          class="w-full bg-white dark:bg-brownish-800 border border-paper-300 dark:border-brownish-700 rounded-lg px-4 py-2 focus:outline-none focus:ring-2 focus:ring-offset-0 focus:ring-redish-400/60 dark:focus:ring-redish-500/50"
        />
      </label>
    </template>

    <template v-else v-for="file in selected" :key="file.id">
      <p>
        {{
          file?.mime === 'dir'
            ? $t('files.delete.confirmDirectory', { name: file?.name })
            : $t('files.delete.confirmFile', { name: file?.name })
        }}
      </p>
    </template>
  </CardBoxModal>
</template>
