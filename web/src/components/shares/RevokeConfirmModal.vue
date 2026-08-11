<script setup lang="ts">
import { useI18n } from 'vue-i18n'
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

const { t } = useI18n()

const title = (): string => {
  if (props.isSelfRemove) return t('shares.revokeModal.selfTitle')
  return t('shares.revokeModal.title')
}

const buttonLabel = (): string => {
  if (props.isSelfRemove) return t('shares.revokeModal.leave')
  return t('shares.revoke')
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
          :label="$t('common.cancel')"
          color="light"
          data-testid="revoke-confirm-modal-cancel"
          @click="emit('cancel')"
        />
      </BaseButtons>
    </template>
    <div data-testid="revoke-confirm-modal">
      <i18n-t
        v-if="isSelfRemove"
        tag="p"
        keypath="shares.revokeModal.selfBody"
        scope="global"
        data-testid="revoke-confirm-modal-self"
      >
        <template #item>
          <strong>{{ itemLabel ?? $t('shares.revokeModal.thisShare') }}</strong>
        </template>
      </i18n-t>
      <i18n-t
        v-else
        tag="p"
        keypath="shares.revokeModal.body"
        scope="global"
        data-testid="revoke-confirm-modal-body"
      >
        <template #recipient><strong>{{ recipientLabel }}</strong></template>
        <template #item>
          <strong>{{ itemLabel ?? $t('shares.revokeModal.thisShare') }}</strong>
        </template>
      </i18n-t>
      <i18n-t
        v-if="!isSelfRemove && cascadeCount && cascadeCount > 0"
        tag="p"
        keypath="shares.revokeModal.cascade"
        :plural="cascadeCount"
        scope="global"
        class="mt-2"
        data-testid="revoke-confirm-modal-cascade"
      >
        <template #count><strong>{{ cascadeCount }}</strong></template>
      </i18n-t>
    </div>
  </CardBoxModal>
</template>
