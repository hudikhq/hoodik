<script setup lang="ts">
import LayoutAuthenticatedWithLoader from '@/layouts/LayoutAuthenticatedWithLoader.vue'
import SectionMain from '@/components/ui/SectionMain.vue'
import MyDetails from './index/MyDetails.vue'
// import CardBox from '@/components/ui/CardBox.vue'
// import CardBoxComponentHeader from '@/components/ui/CardBoxComponentHeader.vue'
</script>
<template>
  <LayoutAuthenticatedWithLoader v-slot="{ authenticated }">
    <Suspense>
      <SectionMain v-if="authenticated">
        <div class="flex space-x-2">
          <MyDetails :user="authenticated.user" />

          <!-- <CardBox class="sm:w-1/2">
            <CardBoxComponentHeader title="Storage usage" class="mb-4" />
          </CardBox> -->
        </div>
      </SectionMain>
    </Suspense>
  </LayoutAuthenticatedWithLoader>
</template>
