<script setup lang="ts">
import LayoutAuthenticatedWithLoader from './LayoutAuthenticatedWithLoader.vue'
import LayoutFileBrowserInner from './components/LayoutFileBrowserInner.vue'

const props = defineProps<{
  parentId?: string
  hideDelete?: boolean
  share?: boolean
  clear?: boolean
}>()
</script>
<template>
  <LayoutAuthenticatedWithLoader :clear="props.clear" v-slot="{ authenticated, keypair }">
    <LayoutFileBrowserInner
      :parentId="props.parentId"
      :hideDelete="props.hideDelete"
      :share="props.share"
      :authenticated="authenticated"
      :keypair="keypair"
      v-slot="all"
    >
      <slot v-bind="all" />
    </LayoutFileBrowserInner>
  </LayoutAuthenticatedWithLoader>
</template>
