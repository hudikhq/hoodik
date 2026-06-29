<script setup lang="ts">
import { useRoute, useRouter } from 'vue-router'
import { store as style } from '!/style'
import { store as login } from '!/auth/login'
import { ensureAuthenticated } from '!/auth'
import { store as cryptoStore } from '!/crypto'
import { useAuthedShareBootstrap } from '@/composables/useAuthedShareBootstrap'

const styleStore = style()
const router = useRouter()
const route = useRoute()

await ensureAuthenticated(router, route)
const crypto = cryptoStore()
const loginStore = login()
useAuthedShareBootstrap()
</script>

<template>
  <div :class="{ dark: styleStore.darkMode }">
    <div class="bg-brownish-50 text-brownish-900 dark:bg-brownish-800 dark:text-dirty-white">
      <slot :authenticated="loginStore.authenticated" :keypair="crypto.keypair" />
    </div>
  </div>
</template>
