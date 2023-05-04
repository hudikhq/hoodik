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

const login = store()
const crypto = cryptoStore()
const router = useRouter()
const hasAuth = ref(true)
const locked = ref(false)

const logout = async () => {
  if (!cryptfns.hasEncryptedPrivateKey() && hasAuthentication(login)) {
    return setTimeout(() => {
      router.push({ path: '/auth/setup-lock-screen', replace: true })
    }, 5000)
  } else if (!cryptfns.hasEncryptedPrivateKey()) {
    hasAuth.value = false
    return setTimeout(() => {
      router.push({ path: '/auth/login', replace: true })
    }, 5000)
  }

  locked.value = true

  try {
    if (hasAuthentication(login)) {
      await login.logout(crypto)
    }
  } catch (e) {
    //
  }
}

logout()

const deletePrivateKey = () => {
  cryptfns.clear()
  router.push({ path: '/auth/login', replace: true })
}
</script>
<template>
  <LayoutGuest v-if="login && crypto">
    <SectionFullScreen v-slot="{ cardClass }" bg="pinkRed">
      <CardBox :class="`${cardClass} text-center`" v-if="!locked && hasAuth">
        <h1 class="text-2xl text-white mb-5">Account Locked</h1>

        <div class="flex items-start">
          <div class="flex items-center h-5">
            <p class="text-sm">
              Looks like you don't have your private key stored locally. You will be automatically
              redirected to the setup lock screen page in a few seconds.
            </p>
          </div>
        </div>

        <router-link
          to="/auth/setup-lock-screen"
          class="float-right rounded-md text-green-200 py-2 px-4 border border-green-300"
        >
          Setup Lock Screen
        </router-link>
      </CardBox>
      <CardBox :class="`${cardClass} text-center`" v-else-if="!locked && !hasAuth">
        <div class="flex items-start">
          <div class="flex items-center h-5">
            <p class="text-sm">
              You are not authenticated, you will be automatically redirected to the login page in a
              few seconds.
            </p>
          </div>
        </div>

        <router-link
          to="/auth/login"
          class="float-right rounded-md text-green-200 py-2 px-4 border border-green-300"
        >
          Login
        </router-link>
      </CardBox>
      <CardBox :class="`${cardClass} text-center`" v-else>
        <div class="h-5 text-center">
          <p class="text-sm">
            Your private key is encrypted, all other data has been deleted and your account has been
            locked.
          </p>
        </div>

        <div class="text-center mt-10 mb-10">
          <BaseIcon :path="mdiLock" size="150" w="w-50" h="h-50" class="text-red-200" />
        </div>

        <router-link
          to="/auth/decrypt"
          class="float-left rounded-md text-green-200 py-2 px-4 border border-green-300"
        >
          Unlock Account
        </router-link>

        <AppButton
          @click="deletePrivateKey"
          type="button"
          class="float-right rounded-md text-red-200 py-2 px-4 border border-red-300"
        >
          Delete Encrypted Private Key
        </AppButton>
      </CardBox>
    </SectionFullScreen>
  </LayoutGuest>
</template>
