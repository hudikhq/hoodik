<script setup lang="ts">
import { useRoute, useRouter } from 'vue-router'
import { store as style } from '!/style'
import { store as login } from '!/auth/login'
import { ensureAuthenticated } from '!/auth'
import { store as cryptoStore } from '!/crypto'

const crypto = cryptoStore()
const styleStore = style()
const router = useRouter()
const route = useRoute()
const loginStore = login()

await ensureAuthenticated(router, route)
</script>

<template>
  <div :class="{ dark: styleStore.darkMode }">
    <div class="bg-brownish-50 dark:bg-brownish-800 dark:text-brownish-100">
      <slot :authenticated="loginStore.authenticated" :keypair="crypto.keypair" />
    </div>
  </div>
</template>
