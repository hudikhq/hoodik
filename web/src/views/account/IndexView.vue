<script setup lang="ts">
import LayoutAuthenticatedWithLoader from '@/layouts/LayoutAuthenticatedWithLoader.vue'
import SectionMain from '@/components/ui/SectionMain.vue'
import MyDetails from './index/MyDetails.vue'
import StorageStats from './index/StorageStats.vue'
</script>
<template>
  <LayoutAuthenticatedWithLoader v-slot="{ authenticated }">
    <Suspense>
      <SectionMain v-if="authenticated">
        <div class="flex space-x-2">
          <MyDetails :user="authenticated.user" />
          <StorageStats />
        </div>
      </SectionMain>
    </Suspense>
  </LayoutAuthenticatedWithLoader>
</template>
