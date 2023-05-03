<script setup lang="ts">
import { ref } from 'vue'
import { mdiClose, mdiDotsVertical } from '@mdi/js'
import { containerMaxW } from '@/config.js'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import NavBarMenuList from '@/components/ui/NavBarMenuList.vue'
import NavBarItemPlain from '@/components/ui/NavBarItemPlain.vue'
import type { NavBarItem } from '@/menuNavBar'

defineProps({
  menu: {
    type: Array<Object>,
    required: true
  }
})

const emit = defineEmits(['menu-click'])

const menuClick = (event: Event, item: NavBarItem) => {
  emit('menu-click', event, item)
}

const isMenuNavBarActive = ref(false)
</script>

<template>
  <nav
    class="top-0 inset-x-0 fixed bg-brownish-50 h-14 z-30 transition-position w-screen lg:w-auto dark:bg-brownish-800"
  >
    <div class="flex lg:items-stretch" :class="containerMaxW">
      <div class="flex flex-1 items-stretch h-14">
        <slot />
      </div>
      <div class="flex-none items-stretch flex h-14 lg:hidden">
        <NavBarItemPlain @click.prevent="isMenuNavBarActive = !isMenuNavBarActive">
          <BaseIcon :path="isMenuNavBarActive ? mdiClose : mdiDotsVertical" size="24" />
        </NavBarItemPlain>
      </div>
      <div
        class="max-h-screen-menu overflow-y-auto lg:overflow-visible absolute w-screen top-14 left-0 bg-brownish-50 shadow-lg lg:w-auto lg:flex lg:static lg:shadow-none dark:bg-brownish-800"
        :class="[isMenuNavBarActive ? 'block' : 'hidden']"
      >
        <NavBarMenuList :menu="menu" @menu-click="menuClick" />
      </div>
    </div>
  </nav>
</template>
