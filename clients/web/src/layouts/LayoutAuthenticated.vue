<script setup lang="ts">
import { mdiForwardburger, mdiBackburger, mdiMenu } from '@mdi/js'
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import menuAside from '@/menuAside'
import menuNavBar, { type NavBarItem } from '@/menuNavBar'
import { store as style } from '@/stores/style'
import { store as login } from '@/stores/auth/login'
import { ensureAuthenticated } from '@/stores/auth'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import NavBar from '@/components/ui/NavBar.vue'
import NavBarItemPlain from '@/components/ui/NavBarItemPlain.vue'
import AsideMenu from '@/components/ui/AsideMenu.vue'
import StatusBar from '@/components/files/io/StatusBar.vue'
import { store as cryptoStore } from '@/stores/crypto'
import SearchButton from '@/components/files/search/SearchButton.vue'
import SearchModal from '@/components/files/search/SearchModal.vue'

const crypto = cryptoStore()
const styleStore = style()
const router = useRouter()
const loginStore = login()

await ensureAuthenticated(router, loginStore, crypto)

const layoutAsidePadding = 'xl:pl-60'

const isAsideMobileExpanded = ref(false)
const isAsideLgActive = ref(false)
const isSearchModalActive = ref(false)

router.beforeEach(async () => {
  isAsideMobileExpanded.value = false
  isAsideLgActive.value = false
})

const menuClick = (event: Event, item: NavBarItem) => {
  if (item.isToggleLightDark) {
    styleStore.setDarkMode()
  }
}
</script>

<template>
  <div
    :class="{
      dark: styleStore.darkMode,
      'overflow-hidden lg:overflow-visible': isAsideMobileExpanded
    }"
  >
    <div
      :class="[layoutAsidePadding, { 'ml-60 lg:ml-0': isAsideMobileExpanded }]"
      class="pt-5 min-h-screen w-screen transition-position lg:w-auto bg-white dark:bg-brownish-800 dark:text-brownish-100"
    >
      <NavBar
        :menu="menuNavBar"
        :class="[layoutAsidePadding, { 'ml-60 lg:ml-0': isAsideMobileExpanded }]"
        @menu-click="menuClick"
      >
        <NavBarItemPlain
          display="flex lg:hidden"
          @click.prevent="isAsideMobileExpanded = !isAsideMobileExpanded"
        >
          <BaseIcon :path="isAsideMobileExpanded ? mdiBackburger : mdiForwardburger" size="24" />
        </NavBarItemPlain>
        <NavBarItemPlain display="hidden lg:flex xl:hidden" @click.prevent="isAsideLgActive = true">
          <BaseIcon :path="mdiMenu" size="24" />
        </NavBarItemPlain>
        <NavBarItemPlain use-margin>
          <SearchButton @search="isSearchModalActive = !isSearchModalActive" />
        </NavBarItemPlain>
      </NavBar>
      <AsideMenu
        :is-aside-mobile-expanded="isAsideMobileExpanded"
        :is-aside-lg-active="isAsideLgActive"
        :menu="menuAside"
        @menu-click="menuClick"
        @aside-lg-close-click="isAsideLgActive = false"
      />
      <SearchModal :keypair="crypto.keypair" v-model="isSearchModalActive" />
      <slot :authenticated="loginStore.authenticated" :keypair="crypto.keypair" />

      <StatusBar />
    </div>
  </div>
</template>
