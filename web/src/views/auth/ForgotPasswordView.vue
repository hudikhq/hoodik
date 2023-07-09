<script setup lang="ts">
import LayoutGuest from '@/layouts/LayoutGuest.vue'
import SectionFullScreen from '@/components/ui/SectionFullScreen.vue'
import CardBox from '@/components/ui/CardBox.vue'
import ChangePasswordForm from '@/components/account/ChangePasswordForm.vue'
</script>
<template>
  <LayoutGuest>
    <SectionFullScreen v-slot="{ cardClass }" bg="pinkRed">
      <CardBox :class="cardClass">
        <h1 class="text-2xl text-white">Recover your account</h1>
        <ChangePasswordForm :forgotPassword="true" />

        <div class="text-sm font-medium text-brownish-500 dark:text-brownish-400 pt-2">
          Know your password?
          <router-link
            :to="{ name: 'login' }"
            class="text-primary-700 hover:underline dark:text-primary-500"
            >Login</router-link
          >
        </div>
      </CardBox>
    </SectionFullScreen>
  </LayoutGuest>
</template>
