<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import BaseButtons from '@/components/ui/BaseButtons.vue'

const props = defineProps<{
  modelValue: boolean
  recipientEmail: string
  cachedFingerprint: string
  newFingerprint: string
  /** Seconds since epoch when the cached fingerprint was last verified.
   *  Helps the user decide whether the gap explains the change (e.g.
   *  "verified 6 months ago" + a known device change = plausible
   *  rotation; "verified yesterday" + a different fingerprint today =
   *  almost certainly substitution). */
  lastVerifiedAt: number
}>()

const emit = defineEmits<{
  (event: 'update:modelValue', value: boolean): void
  (event: 'accept'): void
  (event: 'cancel'): void
}>()

const { t } = useI18n()

const verifiedAgeLabel = computed(() => {
  const ageSeconds = Math.floor(Date.now() / 1000) - props.lastVerifiedAt
  const days = Math.floor(ageSeconds / (24 * 60 * 60))
  if (days <= 0) return t('shares.fingerprint.today')
  if (days === 1) return t('shares.fingerprint.yesterday')
  if (days < 30) return t('shares.fingerprint.daysAgo', { count: days })
  const months = Math.floor(days / 30)
  if (months < 12) return t('shares.fingerprint.monthsAgo', months)
  const years = Math.floor(days / 365)
  return t('shares.fingerprint.yearsAgo', years)
})
</script>

<template>
  <CardBoxModal
    :title="$t('shares.fingerprint.title')"
    button="danger"
    :has-cancel="true"
    :model-value="modelValue"
    @update:model-value="(v) => emit('update:modelValue', v)"
    @cancel="emit('cancel')"
    @confirm="emit('accept')"
  >
    <template #buttons>
      <BaseButtons>
        <BaseButton
          :label="$t('shares.fingerprint.accept')"
          color="danger"
          data-testid="fingerprint-mismatch-accept"
          @click="emit('accept')"
        />
        <BaseButton
          :label="$t('common.cancel')"
          color="light"
          data-testid="fingerprint-mismatch-cancel"
          @click="emit('cancel')"
        />
      </BaseButtons>
    </template>

    <div data-testid="fingerprint-mismatch-modal" class="space-y-3 text-sm">
      <i18n-t tag="p" keypath="shares.fingerprint.changed" scope="global">
        <template #email><strong>{{ recipientEmail }}</strong></template>
        <template #age><strong>{{ verifiedAgeLabel }}</strong></template>
      </i18n-t>
      <p>
        {{ $t('shares.fingerprint.explanation') }}
      </p>
      <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 mt-2">
        <div>
          <div class="text-xs font-medium text-brownish-300 dark:text-brownish-50">
            {{ $t('shares.fingerprint.cachedLabel') }}
          </div>
          <div
            class="font-mono text-xs break-all"
            data-testid="fingerprint-mismatch-cached"
          >
            {{ cachedFingerprint }}
          </div>
        </div>
        <div>
          <div class="text-xs font-medium text-redish-600 dark:text-redish-100">
            {{ $t('shares.fingerprint.newLabel') }}
          </div>
          <div
            class="font-mono text-xs break-all"
            data-testid="fingerprint-mismatch-new"
          >
            {{ newFingerprint }}
          </div>
        </div>
      </div>
      <p class="mt-2 text-brownish-300 dark:text-brownish-50">
        {{ $t('shares.fingerprint.confirmHint', { email: recipientEmail }) }}
      </p>
    </div>
  </CardBoxModal>
</template>
