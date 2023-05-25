<script setup lang="ts">
import LayoutFileBrowser from '@/layouts/LayoutFileBrowser.vue'
import PreviewViewInner from './components/PreviewViewInner.vue'
</script>
<template>
  <LayoutFileBrowser clear v-slot="{ keypair, Storage, on }">
    <PreviewViewInner :kp="keypair" :Storage="Storage" v-on="on" />
  </LayoutFileBrowser>
</template>
