<script setup lang="ts">
import LayoutGuest from '@/layouts/LayoutGuest.vue'
import SectionFullScreen from '@/components/ui/SectionFullScreen.vue'
import CardBox from '@/components/ui/CardBox.vue'
import { store } from '!/auth/login'
import { store as cryptoStore } from '!/crypto'
import { mdiLock } from '@mdi/js'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import * as cryptfns from '!/cryptfns'
import { useRouter } from 'vue-router'
import { ref } from 'vue'
import { hasAuthentication } from '!/auth'
import { AppButton } from '@/components/form'
import BaseButton from '@/components/ui/BaseButton.vue'

const login = store()
const crypto = cryptoStore()
const router = useRouter()
const state = ref<undefined | 'secured' | 'unauthenticated' | 'authenticated'>()

const logout = () => {
  if (!cryptfns.hasEncryptedPrivateKey() && hasAuthentication(login)) {
    state.value = 'authenticated'

    return setTimeout(() => {
      router.push({ name: 'setup-lock-screen', replace: true })
    }, 5000)
  } else if (!cryptfns.hasEncryptedPrivateKey()) {
    state.value = 'unauthenticated'

    return setTimeout(() => {
      router.push({ name: 'login', replace: true })
    }, 5000)
  }

  state.value = 'secured'

  try {
    if (hasAuthentication(login)) {
      login.logout(crypto)
    }
  } catch (e) {
    //
  }
}

logout()

const deletePrivateKey = () => {
  cryptfns.clear()
  router.push({ name: 'login', replace: true })
}
</script>
<template>
  <LayoutGuest v-if="login && crypto">
    <SectionFullScreen v-slot="{ cardClass }" bg="pinkRed">
      <CardBox :class="`${cardClass} text-center`" v-if="state === 'authenticated'">
        <h1 class="text-2xl text-white mb-5">Account Locked</h1>

        <div class="flex items-start">
          <div class="flex items-center mb-4">
            <p class="text-sm">
              Looks like you don't have your private key stored locally. You will be automatically
              redirected to the setup lock screen page in a few seconds.
            </p>
          </div>
        </div>

        <BaseButton :to="{ name: 'setup-lock-screen' }" label="Setup Lock Screen" />
      </CardBox>

      <CardBox :class="`${cardClass} text-center`" v-if="state === 'unauthenticated'">
        <div class="flex items-start">
          <div class="flex items-center mb-4">
            <p class="text-sm block">
              You are not authenticated, you will be automatically redirected to the login page in a
              few seconds.
            </p>
          </div>
        </div>

        <BaseButton :to="{ name: 'login' }" label="login" />
      </CardBox>
      <CardBox :class="`${cardClass} text-center`" v-if="state === 'secured'">
        <div class="h-5 text-center">
          <p class="text-sm">
            Your private key is encrypted, all other data has been deleted and your account has been
            locked.
          </p>
        </div>

        <div class="text-center mt-10 mb-10">
          <BaseIcon :path="mdiLock" size="150" w="w-50" h="h-50" class="text-greeny-400" />
        </div>

        <BaseButton :to="{ name: 'decrypt' }" class="float-left" label="Unlock" />

        <AppButton @click="deletePrivateKey" type="button" class="float-right" color="danger">
          Forget account
        </AppButton>
      </CardBox>
    </SectionFullScreen>
  </LayoutGuest>
</template>
