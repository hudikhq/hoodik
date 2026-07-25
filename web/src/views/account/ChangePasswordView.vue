<script setup lang="ts">
import LayoutAuthenticatedWithLoader from '@/layouts/LayoutAuthenticatedWithLoader.vue'
import SectionMain from '@/components/ui/SectionMain.vue'
import CardBox from '@/components/ui/CardBox.vue'
import CardBoxComponentHeader from '@/components/ui/CardBoxComponentHeader.vue'
import ChangePasswordForm from '@/components/account/ChangePasswordForm.vue'
</script>
<template>
  <LayoutAuthenticatedWithLoader v-slot="{ authenticated, keypair }">
    <Suspense>
      <SectionMain v-if="authenticated">
        <div class="flex justify-center">
          <CardBox class="w-full sm:w-1/2">
            <CardBoxComponentHeader :title="$t('account.changePassword.title')" />

            <ChangePasswordForm :email="authenticated.user.email" :keypair="keypair" />
          </CardBox>
        </div>
      </SectionMain>
    </Suspense>
  </LayoutAuthenticatedWithLoader>
</template>
