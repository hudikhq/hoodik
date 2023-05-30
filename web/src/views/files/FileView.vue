<script setup lang="ts">
import LayoutFileBrowser from '@/layouts/LayoutFileBrowser.vue'
import FileView from './file-view/FileView.vue'
</script>
<template>
  <LayoutFileBrowser clear v-slot="{ keypair, Storage, on }">
    <FileView :kp="keypair" :Storage="Storage" v-on="on" />
  </LayoutFileBrowser>
</template>
