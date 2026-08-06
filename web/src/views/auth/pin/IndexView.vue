<script setup lang="ts">
import LayoutGuest from '@/layouts/LayoutGuest.vue'
import SectionFullScreen from '@/components/ui/SectionFullScreen.vue'
import CardBox from '@/components/ui/CardBox.vue'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import { store } from '!/auth/login'
import { store as cryptoStore } from '!/crypto'
import { mdiLock } from '@mdi/js'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import { useRouter } from 'vue-router'
import { computed, ref } from 'vue'
import { hasAuthentication, pk } from '!/auth'
import * as logger from '!/logger'
import { AppButton } from '@/components/form'
import BaseButton from '@/components/ui/BaseButton.vue'

const login = store()
const crypto = cryptoStore()
const router = useRouter()

/**
 * State of the private key and account
 */
const state = computed<undefined | 'secured' | 'authenticated-secured' | 'authenticated' | 'none'>(
  () => {
    if (pk.hasPin() && hasAuthentication(login)) {
      return 'authenticated-secured'
    } else if (!pk.hasPin() && hasAuthentication(login)) {
      return 'authenticated'
    } else if (!pk.hasPin() && !hasAuthentication(login)) {
      return 'none'
    }

    return 'secured'
  }
)

/**
 * Email of the account that has stored private key with pin
 */
const email = computed(() => {
  const e = pk.getPinEmail()

  return e || undefined
})

/**
 * Handle various states and redirects to proper pages
 */
const handle = () => {
  logger.debug('pin state', state.value)

  if (state.value === 'authenticated-secured') {
    login.logout(crypto, false)
    router.push({ name: 'decrypt', replace: true })
  } else if (state.value === 'authenticated') {
    router.push({ name: 'setup-lock-screen', replace: true })
  } else if (state.value === 'none') {
    router.push({ name: 'login', replace: true })
  } else {
    login.logout(crypto, false)
  }
}

/**
 * Forget the stored private key and redirect to login page
 */
const forgetConfirm = ref(false)

const forget = async () => {
  pk.clearPin()
  router.push({ name: 'login', replace: true })
}

handle()
</script>
<template>
  <LayoutGuest>
    <SectionFullScreen v-slot="{ cardClass }" bg="pinkRed">
      <CardBox :class="`${cardClass} text-center`">
        <div class="h-5 text-center">
          <p class="text-sm">
            {{ $t('auth.lock.description', { email }) }}
          </p>
        </div>

        <div class="text-center mt-10 mb-10">
          <BaseIcon :path="mdiLock" size="150" w="w-50" h="h-50" class="text-greeny-400" />
        </div>

        <BaseButton :to="{ name: 'decrypt' }" class="float-left" :label="$t('auth.unlock')" />

        <AppButton @click="forgetConfirm = true" type="button" class="float-right" color="danger">
          {{ $t('auth.lock.forgetAccount') }}
        </AppButton>

        <CardBoxModal
          v-model="forgetConfirm"
          :title="$t('auth.lock.forgetConfirmTitle')"
          button="danger"
          :button-label="$t('auth.lock.forgetConfirmLabel')"
          :has-cancel="true"
          @confirm="forget"
        >
          {{ $t('auth.lock.forgetConfirmBody') }}
        </CardBoxModal>
      </CardBox>
    </SectionFullScreen>
  </LayoutGuest>
</template>
