<script setup lang="ts">
import LayoutFileBrowser from '@/layouts/LayoutFileBrowser.vue'
import NotesLayout from './NotesLayout.vue'
</script>

<template>
  <LayoutFileBrowser :clear="false" :hide-delete="false" :share="false" v-slot="{ Storage, keypair, loading }">
    <NotesLayout :Storage="Storage" :keypair="keypair" :loading="loading" />
  </LayoutFileBrowser>
</template>
