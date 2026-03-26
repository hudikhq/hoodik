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
  mdiCheck
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
    <!-- Identity header -->
    <div class="-mx-4 -mt-4 px-6 py-5 border-b border-brownish-100 dark:border-brownish-700/50">
      <div class="flex items-center gap-4">
        <div class="w-12 h-12 rounded-full bg-redish-500/15 dark:bg-redish-500/20 text-redish-500 dark:text-redish-400 flex items-center justify-center text-xl font-bold shrink-0 select-none">
          {{ initials }}
        </div>
        <div class="min-w-0 flex-1">
          <p class="font-semibold truncate">{{ user.email }}</p>
          <div class="flex flex-wrap items-center gap-x-2 gap-y-1 mt-1">
            <span v-if="user.role" class="inline-flex items-center text-xs font-semibold uppercase tracking-wider bg-orangy-400/15 text-orangy-400 px-2 py-0.5 rounded-full">
              {{ user.role }}
            </span>
            <span v-else class="inline-flex items-center text-xs bg-brownish-100 dark:bg-brownish-700 text-brownish-400 dark:text-brownish-300 px-2 py-0.5 rounded-full">
              user
            </span>
            <span class="text-xs text-brownish-400">member since {{ createdAt }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Security section -->
    <div class="-mx-4 px-6 py-4 border-b border-brownish-100 dark:border-brownish-700/50">
      <p class="text-xs font-semibold uppercase tracking-wider text-brownish-400 dark:text-brownish-500 mb-4">Security</p>

      <div class="space-y-4">
        <!-- Password -->
        <div class="flex items-center justify-between gap-4">
          <div class="min-w-0">
            <p class="text-sm font-medium">Password</p>
            <p class="text-xs text-brownish-400 mt-0.5">Update your account password</p>
          </div>
          <BaseButton
            :small="true"
            :icon="mdiPassport"
            label="Change"
            :to="{ name: 'account-change-password' }"
            class="shrink-0"
          />
        </div>

        <!-- TFA -->
        <div class="flex items-center justify-between gap-4">
          <div class="min-w-0">
            <div class="flex items-center gap-2">
              <p class="text-sm font-medium">Two-Factor Auth</p>
              <span v-if="user.secret" class="inline-flex items-center gap-1 text-xs font-medium text-greeny-500 dark:text-greeny-400">
                <BaseIcon :path="mdiShieldCheck" :size="14" />
                Enabled
              </span>
              <span v-else class="text-xs text-brownish-400">Not enabled</span>
            </div>
            <p class="text-xs text-brownish-400 mt-0.5">
              {{ user.secret ? 'Your account is protected with a TOTP authenticator app.' : 'Add an extra layer of login security.' }}
            </p>
          </div>
          <BaseButton
            v-if="user.secret"
            :small="true"
            :icon="mdiShieldOff"
            color="danger"
            label="Disable"
            @click="emits('disableTfa')"
            class="shrink-0"
          />
          <BaseButton
            v-else
            :small="true"
            :icon="mdiShieldCheck"
            color="info"
            label="Enable"
            @click="emits('enableTfa')"
            class="shrink-0"
          />
        </div>
      </div>
    </div>

    <!-- Collapsible account details -->
    <div class="-mx-4 -mb-4">
      <button
        @click="showDetails = !showDetails"
        class="w-full flex items-center justify-between px-6 py-3 text-xs font-semibold uppercase tracking-wider text-brownish-400 dark:text-brownish-500 hover:bg-brownish-50 dark:hover:bg-brownish-700/30 transition-colors"
      >
        <span>Account Details</span>
        <BaseIcon :path="mdiChevronDown" :size="16" class="transition-transform duration-200" :class="{ 'rotate-180': showDetails }" />
      </button>

      <div v-if="showDetails" class="px-6 pb-4 space-y-3 border-t border-brownish-100 dark:border-brownish-700/50 pt-3">
        <div class="flex items-start gap-3">
          <span class="w-28 shrink-0 text-xs font-medium uppercase tracking-wider text-brownish-400 dark:text-brownish-500 mt-0.5">Email</span>
          <div class="flex-1 min-w-0">
            <span class="text-sm">{{ user.email }}</span>
            <span v-if="emailVerifiedAt" class="ml-2 text-xs text-greeny-500 dark:text-greeny-400">verified {{ emailVerifiedAt }}</span>
            <span v-else class="ml-2 text-xs text-redish-500">not verified</span>
          </div>
        </div>

        <div class="flex items-start gap-3">
          <span class="w-28 shrink-0 text-xs font-medium uppercase tracking-wider text-brownish-400 dark:text-brownish-500 mt-0.5">Public Key</span>
          <div class="flex-1 min-w-0 flex items-start gap-2">
            <code class="flex-1 font-mono text-xs break-all text-brownish-400 dark:text-brownish-300 leading-relaxed">{{ user.pubkey }}</code>
            <button
              @click="copyToClipboard(user.pubkey, 'pubkey')"
              class="shrink-0 mt-0.5 text-brownish-400 hover:text-brownish-900 dark:hover:text-white transition-colors"
              :title="copiedField === 'pubkey' ? 'Copied!' : 'Copy public key'"
            >
              <BaseIcon :path="copiedField === 'pubkey' ? mdiCheck : mdiContentCopy" :size="14" />
            </button>
          </div>
        </div>

        <div class="flex items-start gap-3">
          <span class="w-28 shrink-0 text-xs font-medium uppercase tracking-wider text-brownish-400 dark:text-brownish-500 mt-0.5">Fingerprint</span>
          <div class="flex-1 min-w-0 flex items-start gap-2">
            <code class="flex-1 font-mono text-xs break-all text-brownish-400 dark:text-brownish-300">{{ user.fingerprint }}</code>
            <button
              @click="copyToClipboard(user.fingerprint, 'fingerprint')"
              class="shrink-0 mt-0.5 text-brownish-400 hover:text-brownish-900 dark:hover:text-white transition-colors"
              :title="copiedField === 'fingerprint' ? 'Copied!' : 'Copy fingerprint'"
            >
              <BaseIcon :path="copiedField === 'fingerprint' ? mdiCheck : mdiContentCopy" :size="14" />
            </button>
          </div>
        </div>
      </div>
    </div>
  </CardBox>
</template>
