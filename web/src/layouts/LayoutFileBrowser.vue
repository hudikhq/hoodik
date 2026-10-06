<script setup lang="ts">
import LayoutAuthenticatedWithLoader from './LayoutAuthenticatedWithLoader.vue'
import LayoutFileBrowserInner from './components/LayoutFileBrowserInner.vue'

const props = defineProps<{
  parentId?: string
  hideDelete?: boolean
  share?: boolean
  clear?: boolean
}>()

// Keeps a stray drop from navigating the browser to the file. It must not stop
// propagation: TableFiles listens on the window to accept drops page-wide.
const drop = (e: DragEvent) => {
  e.preventDefault()
}
</script>
<template>
  <div @drop="drop" @dragover="drop">
    <LayoutAuthenticatedWithLoader :clear="props.clear" v-slot="{ authenticated, keypair }">
      <LayoutFileBrowserInner
        v-if="authenticated"
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
  </div>
</template>
