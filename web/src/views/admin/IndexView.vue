<script setup lang="ts">
import LayoutAdminWithLoader from '@/layouts/LayoutAdminWithLoader.vue'
import StatsCard from './index/StatsCard.vue'
import UsersCard from './index/UsersCard.vue'
import InvitationsCard from './index/InvitationsCard.vue'
import SectionMain from '@/components/ui/SectionMain.vue'
</script>
<template>
  <LayoutAdminWithLoader>
    <Suspense>
      <SectionMain>
        <div class="flex flex-wrap">
          <UsersCard class="w-full p-2" />
          <InvitationsCard class="w-full p-2 md:w-1/2" />
          <StatsCard class="w-full p-2 md:w-1/2" />
        </div>
      </SectionMain>
    </Suspense>
  </LayoutAdminWithLoader>
</template>
