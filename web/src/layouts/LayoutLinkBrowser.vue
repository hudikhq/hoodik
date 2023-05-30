<script setup lang="ts">
import LayoutAuthenticatedWithLoader from './LayoutAuthenticatedWithLoader.vue'
import LayoutLinkBrowserInner from './components/LayoutLinkBrowserInner.vue'

const props = defineProps<{
  clear?: boolean
}>()
</script>
<template>
  <LayoutAuthenticatedWithLoader :clear="props.clear" v-slot="{ authenticated, keypair }">
    <LayoutLinkBrowserInner :authenticated="authenticated" :keypair="keypair" v-slot="all">
      <slot v-bind="all" />
    </LayoutLinkBrowserInner>
  </LayoutAuthenticatedWithLoader>
</template>
