<script lang="ts" setup>
import LinkViewInner from './link-view/LinkViewInner.vue'
import LayoutGuest from '@/layouts/LayoutGuest.vue'
import { store as linksStore } from '!/links'
import { useRoute } from 'vue-router'
import { computed, ref } from 'vue'

const route = useRoute()
const Links = linksStore()

const id = computed((): string => {
  if (Array.isArray(route.params.link_id)) {
    return route.params.link_id[0]
  }

  return route.params.link_id
})

// Capture the link key once from the URL fragment on page load.
// This must NOT be reactive to route.hash — anchor links within
// the markdown preview would overwrite it and destroy the decryption key.
const initialHash = route.hash
const linkKeyHex = ref<string | undefined>(
  initialHash ? initialHash.replace('#', '') : undefined
)
</script>
<template>
  <LayoutGuest>
    <LinkViewInner :Links="Links" :id="id" :linkKeyHex="linkKeyHex" />
  </LayoutGuest>
</template>
