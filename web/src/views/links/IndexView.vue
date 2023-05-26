<script setup lang="ts">
import LayoutLinkBrowser from '@/layouts/LayoutLinkBrowser.vue'
import TableLinks from '@/components/links/list/TableLinks.vue'
import SectionMain from '@/components/ui/SectionMain.vue'
</script>

<template>
  <LayoutLinkBrowser :clear="false" v-slot="{ Links, on }">
    <SectionMain>
      <TableLinks :items="Links.items" :forDelete="Links.forDelete" v-on="on" />
    </SectionMain>
  </LayoutLinkBrowser>
</template>
