<script setup lang="ts">
import { store } from '!/auth/login'
import UserAvatar from '@/components/ui/UserAvatar.vue'

const login = store()
</script>

<template>
  <UserAvatar :email="login.authenticated?.user.email">
    <slot />
  </UserAvatar>
</template>
