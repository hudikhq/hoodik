<script setup lang="ts">
import { mdiClose, mdiLogout } from '@mdi/js'
import { computed } from 'vue'
import { store as style } from '!/style'
import { store as login } from '!/auth/login'
import { store as crypto } from '!/crypto'
import AsideMenuList from '@/components/ui/AsideMenuList.vue'
import AsideMenuItem from '@/components/ui/AsideMenuItem.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import type { AsideMenuItemType } from '@/menuAside'
import { useRouter } from 'vue-router'
import StatsLi from '../files/StatsLi.vue'

defineProps<{
  menu: AsideMenuItemType[]
}>()

const loginStore = login()
const cryptoStore = crypto()
const router = useRouter()

const emit = defineEmits(['menu-click', 'aside-lg-close-click'])

const styleStore = style()

const lockAccountItem = computed(() => ({
  label: 'Logout',
  icon: mdiLogout,
  color: 'info',
  isLogout: true
}))

const name = computed(() => {
  return import.meta.env.APP_NAME || 'Hoodik'
})

const version = computed(() => {
  return import.meta.env.APP_VERSION || ''
})

const menuClick = (event: Event, item: object) => {
  emit('menu-click', event, item)
}

const asideLgCloseClick = (event: Event) => {
  emit('aside-lg-close-click', event)
}

const logoutAction = async () => {
  await loginStore.logout(cryptoStore, true)
  router.push({ name: 'login' })
}
</script>

<template>
  <aside
    id="aside"
    class="lg:py-2 lg:pl-2 w-60 fixed flex z-40 top-0 h-screen transition-position overflow-hidden"
  >
    <div
      :class="styleStore.asideStyle"
      class="lg:rounded-2xl flex-1 flex flex-col overflow-hidden dark:bg-brownish-900"
    >
      <div
        :class="styleStore.asideBrandStyle"
        class="flex flex-row h-14 items-center justify-between dark:bg-brownish-900"
      >
        <div class="text-center flex-1 lg:text-left lg:pl-6 xl:text-center xl:pl-0">
          <b class="font-black">
            {{ name }} <span class="text-xs">{{ version }}</span>
          </b>
        </div>
        <button class="hidden lg:inline-block xl:hidden p-3" @click.prevent="asideLgCloseClick">
          <BaseIcon :path="mdiClose" />
        </button>
      </div>
      <div
        :class="
          styleStore.darkMode ? 'aside-scrollbars-[brownish]' : styleStore.asideScrollbarsStyle
        "
        class="flex-1 overflow-y-auto overflow-x-hidden"
      >
        <AsideMenuList :menu="menu" @menu-click="menuClick" />
      </div>

      <ul>
        <StatsLi />
        <AsideMenuItem :item="lockAccountItem" @menu-click="logoutAction" />
      </ul>

      <div></div>
    </div>
  </aside>
</template>
