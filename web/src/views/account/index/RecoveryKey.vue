<script setup lang="ts">
import { computed, ref } from 'vue'
import { mdiKeyChain, mdiDownload, mdiContentCopy, mdiEye, mdiEyeOff } from '@mdi/js'
import CardBox from '@/components/ui/CardBox.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import { AppButton } from '@/components/form'
import { store as crypto } from '!/crypto'
import { recoveryKeyFor } from '!/auth/bundle'
import { notify } from '@kyvg/vue3-notification'
import { useI18n } from 'vue-i18n'

const props = defineProps<{ class?: string }>()

const { t } = useI18n()

const cryptoStore = crypto()
const revealed = ref(false)

const recoveryKey = computed(() =>
  cryptoStore.keypair ? recoveryKeyFor(cryptoStore.keypair) : ''
)

function download() {
  const blob = new Blob([recoveryKey.value], { type: 'text/plain' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = 'hoodik-recovery-key.txt'
  a.click()
  URL.revokeObjectURL(url)
}

async function copy() {
  await navigator.clipboard.writeText(recoveryKey.value)
  notify({ type: 'success', title: t('common.copied'), text: t('account.recoveryKey.copiedText') })
}
</script>

<template>
  <CardBox :class="props.class" v-if="recoveryKey">
    <div class="flex items-center gap-2 mb-2">
      <BaseIcon :path="mdiKeyChain" />
      <h2 class="text-xl">{{ $t('account.recoveryKey.title') }}</h2>
    </div>

    <i18n-t
      keypath="account.recoveryKey.body"
      tag="p"
      scope="global"
      class="text-sm text-brownish-600 dark:text-dirty-white/70"
    >
      <template #loginWithKey>
        <strong>{{ $t('account.recoveryKey.loginWithKey') }}</strong>
      </template>
    </i18n-t>

    <div class="mt-4 flex flex-wrap gap-2">
      <AppButton
        type="button"
        :icon="revealed ? mdiEyeOff : mdiEye"
        :label="revealed ? $t('account.recoveryKey.hide') : $t('account.recoveryKey.reveal')"
        color="info"
        @click="revealed = !revealed"
      />
      <AppButton
        type="button"
        :icon="mdiDownload"
        :label="$t('common.download')"
        color="success"
        @click="download"
      />
      <AppButton
        type="button"
        :icon="mdiContentCopy"
        :label="$t('common.copy')"
        color="info"
        @click="copy"
      />
    </div>

    <pre
      v-if="revealed"
      class="mt-4 p-3 rounded-lg bg-paper-100 dark:bg-brownish-900 text-xs overflow-x-auto whitespace-pre-wrap break-all"
      >{{ recoveryKey }}</pre
    >
  </CardBox>
</template>
