<script setup lang="ts">
import LayoutAdminWithLoader from '@/layouts/LayoutAdminWithLoader.vue'
import IndexInner from './index/IndexInner.vue'
import UsersInner from './index/UsersInner.vue'
import SectionMain from '@/components/ui/SectionMain.vue'
</script>
<template>
  <LayoutAdminWithLoader>
    <Suspense>
      <SectionMain>
        <div class="flex flex-wrap">
          <UsersInner class="w-full md:w-1/2" />
          <IndexInner class="w-full mt-2 md:mt-0 md:w-1/2 md:ml-2 md:-mr-2" />
        </div>
      </SectionMain>
    </Suspense>
  </LayoutAdminWithLoader>
</template>
