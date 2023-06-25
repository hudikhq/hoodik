<script setup lang="ts">
import AsideMenuLayer from '@/components/ui/AsideMenuLayer.vue'
import OverlayLayer from '@/components/ui/OverlayLayer.vue'
import type { AsideMenuItemType } from '@/menuAside'
import { computed } from 'vue'

const props = defineProps<{
  menu: AsideMenuItemType[]
  isAsideMobileExpanded: boolean
  isAsideLgActive: boolean
  role?: string
}>()

const visibleMenu = computed(() => {
  return props.menu.filter((item) => {
    if (item.roles) {
      return item.roles.some((role) => role === props.role)
    }

    return true
  })
})

const emit = defineEmits(['menu-click', 'aside-lg-close-click'])

const menuClick = (event: Event, item: object) => {
  emit('menu-click', event, item)
}

const asideLgCloseClick = (event: Event) => {
  emit('aside-lg-close-click', event)
}
</script>

<template>
  <AsideMenuLayer
    :menu="visibleMenu"
    :class="[
      isAsideMobileExpanded ? 'left-0' : '-left-60 lg:left-0',
      { 'lg:hidden xl:flex': !isAsideLgActive }
    ]"
    @menu-click="menuClick"
    @aside-lg-close-click="asideLgCloseClick"
  />
  <OverlayLayer v-show="isAsideLgActive" z-index="z-30" @overlay-click="asideLgCloseClick" />
</template>
