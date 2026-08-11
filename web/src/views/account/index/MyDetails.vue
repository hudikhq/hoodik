<script setup lang="ts">
import { formatPrettyDate } from '!/index'
import type { User } from 'types'
import { computed, ref } from 'vue'
import {
  mdiKey,
  mdiPassport,
  mdiShieldCheck,
  mdiShieldOff,
  mdiChevronDown,
  mdiContentCopy,
  mdiCheck,
  mdiLock
} from '@mdi/js'
import CardBox from '@/components/ui/CardBox.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'

const props = defineProps<{
  user: User
  class?: string
}>()

const emits = defineEmits(['disableTfa', 'enableTfa'])

const showDetails = ref(false)
const copiedField = ref<string | null>(null)

const createdAt = computed(() => formatPrettyDate(props.user.created_at))
const emailVerifiedAt = computed(() => {
  if (props.user.email_verified_at) return formatPrettyDate(props.user.email_verified_at)
  return null
})

const initials = computed(() => props.user.email[0]?.toUpperCase() || '?')

async function copyToClipboard(value: string, field: string) {
  await navigator.clipboard.writeText(value)
  copiedField.value = field
  setTimeout(() => { copiedField.value = null }, 2000)
}
</script>
<template>
  <CardBox :class="props.class" v-if="user">
    <div class="-mx-4 -mt-4 px-6 py-6 bg-gradient-to-r from-redish-500/5 via-transparent to-transparent dark:from-redish-500/10 border-b border-paper-200 dark:border-brownish-700/50 rounded-t-2xl">
      <div class="flex items-center gap-4">
        <div class="w-14 h-14 rounded-full bg-gradient-to-br from-redish-500 to-redish-600 text-white flex items-center justify-center text-xl font-bold shrink-0 select-none shadow-lg shadow-redish-500/20">
          {{ initials }}
        </div>
        <div class="min-w-0 flex-1">
          <p class="text-lg font-semibold truncate">{{ user.email }}</p>
          <div class="flex flex-wrap items-center gap-x-2.5 gap-y-1 mt-1.5">
            <span v-if="user.role" class="inline-flex items-center text-xs font-semibold bg-orangy-400/15 text-orangy-400 px-2.5 py-0.5 rounded-full">
              {{ user.role }}
            </span>
            <span v-else class="inline-flex items-center text-xs bg-paper-100 dark:bg-brownish-700 text-brownish-400 dark:text-brownish-50 px-2.5 py-0.5 rounded-full">
              {{ $t('account.details.roleUser') }}
            </span>
            <span class="text-xs text-brownish-400 dark:text-brownish-50">
              {{ $t('account.details.memberSince', { date: createdAt }) }}
            </span>
          </div>
        </div>
      </div>
    </div>

    <div class="-mx-4 px-6 py-5 border-b border-paper-200 dark:border-brownish-700/50">
      <div class="flex items-center gap-2 mb-4">
        <BaseIcon :path="mdiLock" :size="14" class="text-brownish-400 dark:text-brownish-50" />
        <p class="text-xs font-semibold text-brownish-400 dark:text-brownish-50">{{ $t('account.sections.security') }}</p>
      </div>

      <div class="space-y-4">
        <div class="flex items-center justify-between gap-4 p-3 rounded-xl bg-paper-200 dark:bg-brownish-700/20 border border-paper-200/50 dark:border-brownish-700/30">
          <div class="min-w-0">
            <p class="text-sm font-medium">{{ $t('common.password') }}</p>
            <p class="text-xs text-brownish-400 dark:text-brownish-50 mt-0.5">{{ $t('account.details.passwordHint') }}</p>
          </div>
          <BaseButton
            :small="true"
            :icon="mdiPassport"
            :label="$t('account.details.change')"
            :to="{ name: 'account-change-password' }"
            class="shrink-0"
          />
        </div>

        <div class="flex items-center justify-between gap-4 p-3 rounded-xl bg-paper-200 dark:bg-brownish-700/20 border border-paper-200/50 dark:border-brownish-700/30">
          <div class="min-w-0">
            <div class="flex items-center gap-2">
              <p class="text-sm font-medium">{{ $t('account.details.twoFactor') }}</p>
              <span v-if="user.secret" class="inline-flex items-center gap-1 text-xs font-semibold bg-greeny-500/15 text-greeny-500 dark:text-greeny-300 px-2 py-0.5 rounded-full">
                <BaseIcon :path="mdiShieldCheck" :size="12" />
                {{ $t('account.details.enabled') }}
              </span>
              <span v-else class="inline-flex items-center text-xs bg-paper-100 dark:bg-brownish-700 text-brownish-400 dark:text-brownish-50 px-2 py-0.5 rounded-full">
                {{ $t('account.details.notEnabled') }}
              </span>
            </div>
            <p class="text-xs text-brownish-400 dark:text-brownish-50 mt-0.5">
              {{ user.secret ? $t('account.details.tfaOnHint') : $t('account.details.tfaOffHint') }}
            </p>
          </div>
          <BaseButton
            v-if="user.secret"
            :small="true"
            :icon="mdiShieldOff"
            color="danger"
            :label="$t('account.tfa.disable')"
            @click="emits('disableTfa')"
            class="shrink-0"
          />
          <BaseButton
            v-else
            :small="true"
            :icon="mdiShieldCheck"
            color="info"
            :label="$t('account.tfa.enable')"
            @click="emits('enableTfa')"
            class="shrink-0"
          />
        </div>
      </div>
    </div>

    <div class="-mx-4 -mb-4">
      <button
        @click="showDetails = !showDetails"
        class="w-full flex items-center justify-between px-6 py-3.5 text-xs font-semibold text-brownish-400 dark:text-brownish-50 hover:bg-paper-50 dark:hover:bg-brownish-700/30 transition-colors rounded-b-2xl"
      >
        <div class="flex items-center gap-2">
          <BaseIcon :path="mdiKey" :size="14" />
          <span>{{ $t('account.details.title') }}</span>
        </div>
        <BaseIcon :path="mdiChevronDown" :size="16" class="transition-transform duration-200" :class="{ 'rotate-180': showDetails }" />
      </button>

      <div v-if="showDetails" class="px-6 pb-5 space-y-3 border-t border-paper-200 dark:border-brownish-700/50 pt-4">
        <div class="flex items-start gap-3">
          <span class="w-28 shrink-0 text-xs font-medium text-brownish-400 dark:text-brownish-50 mt-0.5">{{ $t('common.email') }}</span>
          <div class="flex-1 min-w-0">
            <span class="text-sm">{{ user.email }}</span>
            <span v-if="emailVerifiedAt" class="ml-2 inline-flex items-center text-xs bg-greeny-500/15 text-greeny-500 dark:text-greeny-300 px-2 py-0.5 rounded-full">{{ $t('account.details.verified', { date: emailVerifiedAt }) }}</span>
            <span v-else class="ml-2 inline-flex items-center text-xs bg-redish-500/15 text-redish-500 px-2 py-0.5 rounded-full">{{ $t('account.details.notVerified') }}</span>
          </div>
        </div>

        <div class="flex items-start gap-3">
          <span class="w-28 shrink-0 text-xs font-medium text-brownish-400 dark:text-brownish-50 mt-0.5">{{ $t('account.details.publicKey') }}</span>
          <div class="flex-1 min-w-0 flex items-start gap-2">
            <code class="flex-1 font-mono text-xs break-all text-brownish-400 dark:text-brownish-50 leading-relaxed bg-paper-50 dark:bg-brownish-900/50 px-2.5 py-1.5 rounded-lg">{{ user.pubkey }}</code>
            <button
              @click="copyToClipboard(user.pubkey, 'pubkey')"
              class="shrink-0 mt-1 p-1 rounded-md text-brownish-400 dark:text-brownish-50 hover:text-brownish-900 dark:hover:text-white hover:bg-paper-100 dark:hover:bg-brownish-700 transition-colors"
              :title="copiedField === 'pubkey' ? $t('account.details.copiedTooltip') : $t('account.details.copyPublicKey')"
              :aria-label="copiedField === 'pubkey' ? $t('account.details.copiedTooltip') : $t('account.details.copyPublicKey')"
            >
              <BaseIcon :path="copiedField === 'pubkey' ? mdiCheck : mdiContentCopy" :size="14" />
            </button>
          </div>
        </div>

        <div class="flex items-start gap-3">
          <span class="w-28 shrink-0 text-xs font-medium text-brownish-400 dark:text-brownish-50 mt-0.5">{{ $t('account.details.fingerprint') }}</span>
          <div class="flex-1 min-w-0 flex items-start gap-2">
            <code class="flex-1 font-mono text-xs break-all text-brownish-400 dark:text-brownish-50 bg-paper-50 dark:bg-brownish-900/50 px-2.5 py-1.5 rounded-lg">{{ user.fingerprint }}</code>
            <button
              @click="copyToClipboard(user.fingerprint, 'fingerprint')"
              class="shrink-0 mt-1 p-1 rounded-md text-brownish-400 dark:text-brownish-50 hover:text-brownish-900 dark:hover:text-white hover:bg-paper-100 dark:hover:bg-brownish-700 transition-colors"
              :title="copiedField === 'fingerprint' ? $t('account.details.copiedTooltip') : $t('account.details.copyFingerprint')"
              :aria-label="copiedField === 'fingerprint' ? $t('account.details.copiedTooltip') : $t('account.details.copyFingerprint')"
            >
              <BaseIcon :path="copiedField === 'fingerprint' ? mdiCheck : mdiContentCopy" :size="14" />
            </button>
          </div>
        </div>
      </div>
    </div>
  </CardBox>
</template>
