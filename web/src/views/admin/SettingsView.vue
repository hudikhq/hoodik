<script setup lang="ts">
import LayoutAdminWithLoader from '@/layouts/LayoutAdminWithLoader.vue'
import SectionMain from '@/components/ui/SectionMain.vue'
import SettingsInner from './settings/SettingsInner.vue'
</script>
<template>
  <LayoutAdminWithLoader>
    <Suspense>
      <SectionMain>
        <SettingsInner />
      </SectionMain>
    </Suspense>
  </LayoutAdminWithLoader>
</template>
