<script setup lang="ts">
import { store } from '!/auth/register'
import { useRoute, useRouter } from 'vue-router'
import { ref } from 'vue'
import LayoutGuest from '@/layouts/LayoutGuest.vue'
import SectionFullScreen from '@/components/ui/SectionFullScreen.vue'
import CardBox from '@/components/ui/CardBox.vue'
import type { ErrorResponse } from '!/api'
import PuppyLoader from '@/components/ui/PuppyLoader.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import { mdiCheckCircleOutline, mdiAlertOutline } from '@mdi/js'

const register = store()
const router = useRouter()
const route = useRoute()

const working = ref(true)
const error = ref()
const user = ref()

const verify = async () => {
  working.value = true
  let token = route.params?.token

  if (Array.isArray(token)) {
    token = token[0]
  }

  if (!token) {
    error.value = 'Invalid token'
    working.value = false
    return
  }

  try {
    user.value = await register.verifyEmail(token)

    working.value = false
    error.value = ``

    setTimeout(() => {
      router.push({ name: 'login' })
    }, 10000)
  } catch (err) {
    const _error = err as ErrorResponse<any>
    error.value = _error.description
    working.value = false
  }
}

verify()
</script>
<template>
  <LayoutGuest>
    <SectionFullScreen v-slot="{ cardClass }" bg="pinkRed">
      <CardBox :class="`${cardClass} h-[450px] text-center`">
        <PuppyLoader v-model="working" />

        <h1 class="text-2xl text-white">Attempting to verify your email</h1>

        <BaseIcon
          v-if="!error && !working"
          :path="mdiCheckCircleOutline"
          class="text-greeny-400"
          :size="200"
          w="w-full"
          h="h-80"
        />

        <BaseIcon
          v-if="error && !working"
          :path="mdiAlertOutline"
          class="text-redish-400"
          :size="200"
          w="w-full"
          h="h-80"
        />

        <p class="dark:text-white text-brownish-900" v-if="!working && !error">
          <span class="text-greeny-400">Verification successful!</span> <br />
          You will be redirected to login page in a moment.
        </p>

        <p class="dark:text-white text-brownish-900" v-if="error && !working">
          Email verification failed: <br />
          <span class="text-redish-400">{{ error }}</span>
        </p>

        <router-link :to="{ name: 'home' }" class="underline hover:no-underline">
          Go home.
        </router-link>
      </CardBox>
    </SectionFullScreen>
  </LayoutGuest>
</template>
