<script setup lang="ts">
import { computed } from 'vue'
import { mdiCellphoneLink } from '@mdi/js'
import QRCodeComponent from 'qrcode.vue'
import CardBox from '@/components/ui/CardBox.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import { connectUrl, mobilePlatform, openInApp } from '!/connect'
import type { User } from 'types'

const props = defineProps<{ user: User; class?: string }>()

const platform = mobilePlatform()
const url = computed(() => connectUrl(window.location.origin, props.user.email))

function openApp() {
  openInApp(window.location.origin, props.user.email, platform)
}
</script>

<template>
  <CardBox :class="props.class">
    <div class="flex items-center gap-2 mb-2">
      <BaseIcon :path="mdiCellphoneLink" />
      <h2 class="text-xl">{{ $t('account.connectDevice.title') }}</h2>
    </div>

    <div class="flex flex-col sm:flex-row gap-6 items-center sm:items-start">
      <!-- Fixed white plate in both themes: a dark-inverted QR code scans
           badly on most phone cameras. -->
      <div class="shrink-0 rounded-lg bg-white p-3" data-testid="account-connect-qr">
        <QRCodeComponent :value="url" :size="160" render-as="svg" :margin="0" level="M" />
      </div>

      <div>
        <p class="text-sm text-brownish-600 dark:text-dirty-white/70">
          {{ $t('account.connectDevice.body') }}
        </p>

        <!-- Nobody can scan the screen they're holding, so on a phone the same
             handoff becomes a button: the app, or the store without it. -->
        <button
          v-if="platform"
          type="button"
          class="inline-block mt-3 text-sm text-redish-500 dark:text-redish-300 underline"
          data-testid="account-connect-link"
          @click="openApp"
        >
          {{ $t('account.connectDevice.onThisDevice') }}
        </button>

        <p class="mt-3 text-sm text-brownish-600 dark:text-dirty-white/70">
          {{ $t('account.connectDevice.noSecrets') }}
        </p>
      </div>
    </div>
  </CardBox>
</template>
