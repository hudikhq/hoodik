<script setup lang="ts">
import LayoutLinkBrowser from '@/layouts/LayoutLinkBrowser.vue'
import SectionMain from '@/components/ui/SectionMain.vue'
import TableLinks from './index-view/TableLinks.vue'
</script>

<template>
  <LayoutLinkBrowser :clear="false" v-slot="{ Links, on }">
    <SectionMain>
      <TableLinks :items="Links.items" :selected="Links.selected" v-on="on" />
    </SectionMain>
  </LayoutLinkBrowser>
</template>
