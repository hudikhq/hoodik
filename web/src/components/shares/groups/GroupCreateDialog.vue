<script setup lang="ts">
import { computed, ref, watch } from 'vue'

import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import { AppField } from '@/components/form'

import { groups as shareGroups } from '!/shares'
import { errorNotification, notification } from '!/index'

import type { AppShareGroup } from 'types'

const props = defineProps<{
  modelValue: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
  (e: 'created', group: AppShareGroup): void
  (e: 'cancel'): void
}>()

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
    errorText.value = 'Give the group a name.'
    return
  }
  submitting.value = true
  try {
    const group = await shareGroups.createGroup(trimmed)
    notification('Group created', `"${group.name}" is ready to receive members.`, 'success')
    emit('created', group)
    open.value = false
  } catch (err) {
    const message = err instanceof Error ? err.message : 'Failed to create the group'
    errorText.value = /409|conflict|taken/i.test(message)
      ? 'A group with that name already exists.'
      : message
    errorNotification(err)
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
    title="New share group"
    :model-value="open"
    has-cancel
    hide-submit
    @update:model-value="(value) => (open = value)"
    @cancel="cancel"
  >
    <div class="space-y-3">
      <AppField
        name="group-name"
        label="Group name"
        v-model="name"
        :disabled="submitting"
        placeholder="e.g. Marketing team"
        @confirm="submit"
      />
      <p
        v-if="errorText"
        class="text-sm text-redish-700 dark:text-redish-300"
        data-testid="group-create-error"
      >
        {{ errorText }}
      </p>
      <p class="text-xs text-brownish-300">
        Groups are a saved recipient list, so you can share a file with
        everyone in the group at once instead of adding each person by hand.
      </p>
    </div>

    <template #buttons>
      <BaseButton
        label="Create"
        color="info"
        :disabled="submitting || !name.trim()"
        data-testid="group-create-submit"
        @click.prevent="submit"
      />
      <BaseButton label="Cancel" color="info" outline @click.prevent="cancel" />
    </template>
  </CardBoxModal>
</template>
