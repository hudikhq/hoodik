<script setup lang="ts">
import { mdiClose, mdiLogout } from '@mdi/js'
import { computed, ref, watch } from 'vue'
import { store as style } from '!/style'
import { store as login } from '!/auth/login'
import { store as crypto } from '!/crypto'
import AsideMenuList from '@/components/ui/AsideMenuList.vue'
import AsideMenuItem from '@/components/ui/AsideMenuItem.vue'
import AsideFileTree from '@/components/ui/AsideFileTree.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import type { AsideMenuItemType } from '@/menuAside'
import { useRouter } from 'vue-router'
import StatsLi from '../files/StatsLi.vue'

const props = defineProps<{
  menu: AsideMenuItemType[]
}>()

const loginStore = login()
const cryptoStore = crypto()
const router = useRouter()

const EXPANDED_KEY = 'hoodik:sidebar:expandedKey'
const expandedKey = ref<string | null>(sessionStorage.getItem(EXPANDED_KEY))

const hasKeypair = computed(() => !!cryptoStore.keypair)

watch(expandedKey, (v) => {
  if (v) sessionStorage.setItem(EXPANDED_KEY, v)
  else sessionStorage.removeItem(EXPANDED_KEY)
})

// Auto-expand the matching section based on current route
watch(
  () => router.currentRoute.value.name,
  (routeName) => {
    if (!routeName) return
    const name = String(routeName)
    for (const item of props.menu) {
      if (!item.expandable) continue
      const itemRouteName = (item.to as any)?.name
      if (name === itemRouteName || name.startsWith(itemRouteName + '-')) {
        expandedKey.value = itemRouteName
        return
      }
    }
  },
  { immediate: true }
)

function toggleExpand(item: AsideMenuItemType) {
  const key = (item.to as any)?.name
  expandedKey.value = expandedKey.value === key ? null : key
}

const emit = defineEmits(['menu-click', 'aside-lg-close-click'])
const styleStore = style()

const lockAccountItem = computed(() => ({
  label: 'Logout',
  icon: mdiLogout,
  color: 'info',
  isLogout: true
}))

const name = computed(() => import.meta.env.APP_NAME || 'Hoodik')
const version = computed(() => import.meta.env.APP_VERSION || '')

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
    class="lg:py-2 lg:pl-2 w-72 fixed flex z-40 top-0 h-screen transition-position overflow-hidden"
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
        :class="styleStore.darkMode ? 'aside-scrollbars-[brownish]' : styleStore.asideScrollbarsStyle"
        class="flex-1 overflow-y-auto overflow-x-hidden"
      >
        <template v-for="item in menu" :key="(item.to as any)?.name || item.label">
          <template v-if="item.expandable && hasKeypair">
            <AsideMenuList :menu="[item]" @menu-click="() => toggleExpand(item)" />
            <AsideFileTree
              v-if="expandedKey === (item.to as any)?.name"
              :keypair="cryptoStore.keypair!"
            />
          </template>
          <AsideMenuList v-else :menu="[item]" @menu-click="menuClick" />
        </template>
      </div>

      <ul>
        <StatsLi />
        <AsideMenuItem :item="lockAccountItem" @menu-click="logoutAction" />
      </ul>

      <div></div>
    </div>
  </aside>
</template>
