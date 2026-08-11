<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { mdiClose, mdiDotsVertical } from '@mdi/js'
import { containerMaxW } from '@/config.js'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import HelpModal from '@/components/ui/HelpModal.vue'
import NavBarMenuList from '@/components/ui/NavBarMenuList.vue'
import NavBarItemPlain from '@/components/ui/NavBarItemPlain.vue'
import type { NavBarItem } from '@/menuNavBar'

defineProps({
  menu: {
    type: Array<Object>,
    required: true
  }
})

// The help dialog is a second root node, which switches off Vue's automatic
// attribute inheritance. The layout passes this component the aside padding as
// a class, and without it the bar slides under the fixed sidebar — so the bar
// takes $attrs explicitly rather than losing them.
defineOptions({ inheritAttrs: false })

const emit = defineEmits(['menu-click'])

const helpOpen = ref(false)

// Help lives here rather than in each layout so both the authenticated and
// admin shells get the same entry and the same shortcut without duplicating it.
const menuClick = (event: Event, item: NavBarItem) => {
  if (item.isHelp) {
    helpOpen.value = true
    isMenuNavBarActive.value = false
    return
  }

  emit('menu-click', event, item)
}

const onKeydown = (e: KeyboardEvent) => {
  if (e.key !== '/' || !(e.metaKey || e.ctrlKey)) return
  e.preventDefault()
  helpOpen.value = !helpOpen.value
}

onMounted(() => window.addEventListener('keydown', onKeydown))
onUnmounted(() => window.removeEventListener('keydown', onKeydown))

const isMenuNavBarActive = ref(false)
</script>

<template>
  <nav
    v-bind="$attrs"
    class="top-0 inset-x-0 fixed h-14 z-30 transition-position w-screen lg:w-auto
           bg-white/80 backdrop-blur-md border-b border-paper-300/40
           dark:bg-brownish-900/80 dark:border-brownish-700/30"
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
        class="max-h-screen-menu overflow-y-auto lg:overflow-visible absolute w-screen top-14 left-0 shadow-lg lg:w-auto lg:flex lg:static lg:shadow-none
               bg-white/95 dark:bg-brownish-900/95 backdrop-blur-md border-b border-paper-300/30 dark:border-brownish-700/30 lg:border-none"
        :class="[isMenuNavBarActive ? 'block' : 'hidden']"
      >
        <NavBarMenuList :menu="menu" @menu-click="menuClick" />
      </div>
    </div>
  </nav>

  <HelpModal v-model="helpOpen" />
</template>
