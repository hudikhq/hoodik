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
        <h1 class="text-2xl text-brownish-700 dark:text-white">{{ $t('auth.forgotPassword.title') }}</h1>
        <ChangePasswordForm :forgotPassword="true" />

        <div class="text-sm font-medium text-brownish-500 dark:text-brownish-100 pt-2">
          {{ $t('auth.forgotPassword.knowPassword') }}
          <router-link
            :to="{ name: 'login' }"
            class="text-primary-700 hover:underline dark:text-primary-500"
            >{{ $t('common.login') }}</router-link
          >
        </div>
      </CardBox>
    </SectionFullScreen>
  </LayoutGuest>
</template>
