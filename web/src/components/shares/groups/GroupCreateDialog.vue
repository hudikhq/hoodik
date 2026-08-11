<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import { AppField } from '@/components/form'

import { groups as shareGroups } from '!/shares'
import { errorNotification, notification, humanizeError } from '!/index'
import type { ErrorResponse } from '!/api'

import type { AppShareGroup } from 'types'

const props = defineProps<{
  modelValue: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
  (e: 'created', group: AppShareGroup): void
  (e: 'cancel'): void
}>()

const { t } = useI18n()

const open = computed({
  get: () => props.modelValue,
  set: (value: boolean) => emit('update:modelValue', value)
})

const name = ref('')
const submitting = ref(false)
const errorText = ref<string | null>(null)

watch(
  () => props.modelValue,
  (value) => {
    if (value) {
      name.value = ''
      errorText.value = null
      submitting.value = false
    }
  }
)

async function submit(): Promise<void> {
  const trimmed = name.value.trim()
  if (!trimmed) {
    errorText.value = t('shares.groups.nameRequired')
    return
  }
  submitting.value = true
  try {
    const group = await shareGroups.createGroup(trimmed)
    notification(t('shares.groups.createdTitle'), t('shares.groups.createdBody', { name: group.name }), 'success')
    emit('created', group)
    open.value = false
  } catch (err) {
    // The response carries a real status; matching "409" against the
    // message text broke as soon as the wording changed, and leaked that
    // wording to the user on every miss.
    const conflict = (err as ErrorResponse<unknown>)?.status === 409
    errorText.value = conflict ? t('shares.groups.nameExists') : humanizeError(err)
    if (!conflict) errorNotification(err)
  } finally {
    submitting.value = false
  }
}

function cancel(): void {
  emit('cancel')
  open.value = false
}
</script>

<template>
  <CardBoxModal
    v-if="open"
    :title="$t('shares.groups.createTitle')"
    :model-value="open"
    has-cancel
    hide-submit
    @update:model-value="(value) => (open = value)"
    @cancel="cancel"
  >
    <div class="space-y-3">
      <AppField
        name="group-name"
        :label="$t('shares.groups.nameLabel')"
        v-model="name"
        :disabled="submitting"
        :placeholder="$t('shares.groups.namePlaceholder')"
        @confirm="submit"
      />
      <p
        v-if="errorText"
        class="text-sm text-redish-700 dark:text-redish-100"
        data-testid="group-create-error"
      >
        {{ errorText }}
      </p>
      <p class="text-xs text-brownish-300 dark:text-brownish-50">
        {{ $t('shares.groups.createNote') }}
      </p>
    </div>

    <template #buttons>
      <BaseButton
        :label="$t('common.create')"
        color="info"
        :disabled="submitting || !name.trim()"
        data-testid="group-create-submit"
        @click.prevent="submit"
      />
      <BaseButton :label="$t('common.cancel')" color="light" @click.prevent="cancel" />
    </template>
  </CardBoxModal>
</template>
