<script setup lang="ts">
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import BaseButtons from '@/components/ui/BaseButtons.vue'

const props = defineProps<{
  modelValue: boolean
  /** Recipient label rendered in the dialog body. The owner-revoke copy
   *  reads "<email> will lose access"; for self-remove it's unused. */
  recipientLabel: string
  /** Optional item label (file or folder name). Falls back to a generic
   *  "this share" so the dialog still reads cleanly when the caller
   *  doesn't have a name handy (e.g. bulk revoke). */
  itemLabel?: string
  /** When set and > 0, surfaces the cascade disclaimer for Co-owners:
   *  revoking them also drops `cascadeCount` downstream grants. */
  cascadeCount?: number
  /** Switches the copy to first-person for the "Remove yourself"
   *  affordance from /share/with-me — the disclaimer points at the
   *  caller's own already-downloaded copies. */
  isSelfRemove?: boolean
}>()

const emit = defineEmits<{
  (event: 'update:modelValue', value: boolean): void
  (event: 'confirm'): void
  (event: 'cancel'): void
}>()

const title = (): string => {
  if (props.isSelfRemove) return 'Remove yourself from this share?'
  return 'Revoke this share?'
}

const buttonLabel = (): string => {
  if (props.isSelfRemove) return 'Leave'
  return 'Revoke'
}
</script>

<template>
  <CardBoxModal
    :title="title()"
    button="danger"
    :button-label="buttonLabel()"
    :has-cancel="true"
    :model-value="modelValue"
    @update:model-value="(v) => emit('update:modelValue', v)"
    @cancel="emit('cancel')"
    @confirm="emit('confirm')"
  >
    <template #buttons>
      <BaseButtons>
        <BaseButton
          :label="buttonLabel()"
          color="danger"
          data-testid="revoke-confirm-modal-accept"
          @click="emit('confirm')"
        />
        <BaseButton
          label="Cancel"
          color="info"
          outline
          data-testid="revoke-confirm-modal-cancel"
          @click="emit('cancel')"
        />
      </BaseButtons>
    </template>
    <div data-testid="revoke-confirm-modal">
      <p v-if="isSelfRemove" data-testid="revoke-confirm-modal-self">
        You will lose access to
        <strong>{{ itemLabel ?? 'this share' }}</strong>
        on future reads. Anything you've already downloaded stays with you —
        end-to-end encryption can't recall plaintext after it's been decrypted
        on your device.
      </p>
      <p v-else data-testid="revoke-confirm-modal-body">
        <strong>{{ recipientLabel }}</strong> will lose access to
        <strong>{{ itemLabel ?? 'this share' }}</strong>
        on future reads. Anything they've already downloaded is outside our
        reach — end-to-end encryption can't unshare plaintext after it has
        been decrypted on their device.
      </p>
      <p
        v-if="!isSelfRemove && cascadeCount && cascadeCount > 0"
        class="mt-2"
        data-testid="revoke-confirm-modal-cascade"
      >
        They are a Co-owner — revoking them also drops
        <strong>{{ cascadeCount }}</strong>
        downstream grant{{ cascadeCount === 1 ? '' : 's' }} they made under
        this folder.
      </p>
    </div>
  </CardBoxModal>
</template>
