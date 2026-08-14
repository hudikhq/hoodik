<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { mdiCellphone, mdiOpenInNew } from '@mdi/js'
import QRCodeComponent from 'qrcode.vue'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import { AppButton } from '@/components/form'
import { store as loginStore } from '!/auth/login'
import { connectUrl, dismissPrompt, isPromptDismissed, mobilePlatform, openInApp } from '!/connect'

const login = loginStore()
const router = useRouter()
const show = ref(false)

const user = computed(() => login.authenticated?.user)
const platform = mobilePlatform()
const url = computed(() =>
  user.value ? connectUrl(window.location.origin, user.value.email) : ''
)

function refresh() {
  const id = user.value?.id
  show.value = !!id && !isPromptDismissed(id)
}

watch(() => user.value?.id, refresh, { immediate: true })

function dismiss() {
  const id = user.value?.id
  if (id) dismissPrompt(id)
  show.value = false
}

/** Hand off to the app, or to the store when this device hasn't got it. */
function openApp() {
  const email = user.value?.email
  if (!email) return

  dismiss()
  openInApp(window.location.origin, email, platform)
}

function goToAccount() {
  dismiss()
  router.push({ name: 'account' })
}
</script>

<template>
  <CardBoxModal
    v-model="show"
    :title="$t('account.connectDevice.promptTitle')"
    button="success"
    :button-label="
      platform ? $t('account.connectDevice.continueOnWeb') : $t('account.connectDevice.promptDone')
    "
    @confirm="dismiss"
    @cancel="dismiss"
  >
    <!-- On the phone itself a QR code is unusable, so the same handoff becomes
         a button: the app if it's installed, the store if it isn't. -->
    <div v-if="platform" class="space-y-4 text-sm">
      <div class="flex items-start gap-3">
        <BaseIcon :path="mdiCellphone" size="32" class="text-greeny-500 dark:text-greeny-300 shrink-0 mt-1" />
        <p>{{ $t('account.connectDevice.promptBodyMobile') }}</p>
      </div>

      <AppButton
        type="button"
        :icon="mdiOpenInNew"
        :label="$t('account.connectDevice.openApp')"
        color="info"
        data-testid="connect-prompt-open-app"
        @click="openApp"
      />

      <p class="text-brownish-600 dark:text-dirty-white/70">
        {{ $t('account.connectDevice.noSecrets') }}
      </p>
    </div>

    <div v-else class="flex flex-col sm:flex-row gap-5 items-center sm:items-start">
      <!-- Fixed white plate in both themes: a dark-inverted QR code scans
           badly on most phone cameras. -->
      <div class="shrink-0 rounded-lg bg-white p-3" data-testid="connect-prompt-qr">
        <QRCodeComponent :value="url" :size="150" render-as="svg" :margin="0" level="M" />
      </div>

      <div class="space-y-3 text-sm">
        <p>{{ $t('account.connectDevice.promptBody') }}</p>
        <p class="text-brownish-600 dark:text-dirty-white/70">
          {{ $t('account.connectDevice.noSecrets') }}
        </p>
        <button type="button" class="text-redish-500 dark:text-redish-300 underline" @click="goToAccount">
          {{ $t('account.connectDevice.promptLater') }}
        </button>
      </div>
    </div>
  </CardBoxModal>
</template>
