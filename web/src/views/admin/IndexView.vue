<script setup lang="ts">
import LayoutAdminWithLoader from '@/layouts/LayoutAdminWithLoader.vue'
import IndexInner from './index/IndexInner.vue'
</script>
<template>
  <LayoutAdminWithLoader>
    <Suspense>
      <IndexInner />
    </Suspense>
  </LayoutAdminWithLoader>
</template>
