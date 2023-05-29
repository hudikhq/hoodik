<script lang="ts" setup>
import LinkViewInner from './components/LinkViewInner.vue'
import LayoutGuest from '@/layouts/LayoutGuest.vue'
import { store as linksStore } from '!/links'
import { useRoute } from 'vue-router'
import { computed } from 'vue'

const route = useRoute()
const Links = linksStore()

const id = computed((): string => {
  if (Array.isArray(route.params.link_id)) {
    return route.params.link_id[0]
  }

  return route.params.link_id
})

const linkKeyHex = computed((): string | undefined => {
  if (route.hash) {
    return route.hash.replace('#', '')
  }

  return undefined
})
</script>
<template>
  <LayoutGuest>
    <LinkViewInner :Links="Links" :id="id" :linkKeyHex="linkKeyHex" />
  </LayoutGuest>
</template>
