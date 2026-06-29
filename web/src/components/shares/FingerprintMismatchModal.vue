<script setup lang="ts">
import { computed } from 'vue'
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

const verifiedAgeLabel = computed(() => {
  const ageSeconds = Math.floor(Date.now() / 1000) - props.lastVerifiedAt
  const days = Math.floor(ageSeconds / (24 * 60 * 60))
  if (days <= 0) return 'today'
  if (days === 1) return 'yesterday'
  if (days < 30) return `${days} days ago`
  const months = Math.floor(days / 30)
  if (months < 12) return `${months} month${months === 1 ? '' : 's'} ago`
  const years = Math.floor(days / 365)
  return `${years} year${years === 1 ? '' : 's'} ago`
})
</script>

<template>
  <CardBoxModal
    title="Recipient's fingerprint changed"
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
          label="Accept new fingerprint"
          color="danger"
          data-testid="fingerprint-mismatch-accept"
          @click="emit('accept')"
        />
        <BaseButton
          label="Cancel"
          color="info"
          outline
          data-testid="fingerprint-mismatch-cancel"
          @click="emit('cancel')"
        />
      </BaseButtons>
    </template>

    <div data-testid="fingerprint-mismatch-modal" class="space-y-3 text-sm">
      <p>
        The public-key fingerprint <strong>{{ recipientEmail }}</strong> presents now
        is different from the one you verified <strong>{{ verifiedAgeLabel }}</strong>.
      </p>
      <p>
        This happens when the recipient legitimately rotates their key (new
        device, password reset, account recovery) — but it is also exactly
        what a key-substitution attack looks like. The server cannot tell the
        two apart; only you can, by verifying out of band.
      </p>
      <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 mt-2">
        <div>
          <div class="text-xs uppercase tracking-wider text-brownish-300">
            Fingerprint you verified
          </div>
          <div
            class="font-mono text-xs break-all"
            data-testid="fingerprint-mismatch-cached"
          >
            {{ cachedFingerprint }}
          </div>
        </div>
        <div>
          <div class="text-xs uppercase tracking-wider text-redish-600">
            Fingerprint the server returned
          </div>
          <div
            class="font-mono text-xs break-all"
            data-testid="fingerprint-mismatch-new"
          >
            {{ newFingerprint }}
          </div>
        </div>
      </div>
      <p class="mt-2 text-brownish-300">
        Accept only after confirming with {{ recipientEmail }} through a
        channel that does not flow through this Hoodik server.
      </p>
    </div>
  </CardBoxModal>
</template>
