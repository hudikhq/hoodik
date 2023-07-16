<script setup lang="ts">
import { mdiForwardburger, mdiBackburger, mdiMenu } from '@mdi/js'
import { ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import menuAside from '@/menuAside'
import menuNavBar, { type NavBarItem } from '@/menuNavBar'
import { store as styleStore } from '!/style'
import { store as loginStore } from '!/auth/login'
import { ensureAuthenticated } from '!/auth'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import NavBar from '@/components/ui/NavBar.vue'
import NavBarItemPlain from '@/components/ui/NavBarItemPlain.vue'
import AsideMenu from '@/components/ui/AsideMenu.vue'
import StatusBar from '@/components/files/io/StatusBar.vue'
import { store as cryptoStore } from '!/crypto'
import SearchButton from '@/components/files/search/SearchButton.vue'
import SearchModal from '@/components/files/search/SearchModal.vue'
import ResendActivationNotification from './components/ResendActivationNotification.vue'

const router = useRouter()
const route = useRoute()

const style = styleStore()

await ensureAuthenticated(router, route)
const crypto = cryptoStore()
const login = loginStore()

const layoutAsidePadding = 'xl:pl-60'

const isAsideMobileExpanded = ref(false)
const isAsideLgActive = ref(false)
const isSearchModalActive = ref(false)

router.beforeEach(async () => {
  isAsideMobileExpanded.value = false
  isAsideLgActive.value = false
})

const menuClick = (event: Event, item: NavBarItem) => {
  if (item.isTogglelight) {
    style.setDarkMode()
  }
}
</script>

<template>
  <div
    :class="{
      dark: style.darkMode,
      'overflow-hidden lg:overflow-visible': isAsideMobileExpanded
    }"
  >
    <div
      :class="[layoutAsidePadding, { 'ml-60 lg:ml-0': isAsideMobileExpanded }]"
      class="pt-16 min-h-screen w-screen transition-position lg:w-auto bg-white dark:bg-brownish-800 dark:text-brownish-100"
    >
      <NavBar
        v-if="login.authenticated"
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
        v-if="login.authenticated"
        :is-aside-mobile-expanded="isAsideMobileExpanded"
        :is-aside-lg-active="isAsideLgActive"
        :role="login.authenticated?.user?.role"
        :menu="menuAside"
        @menu-click="menuClick"
        @aside-lg-close-click="isAsideLgActive = false"
      />
      <SearchModal :keypair="crypto.keypair" v-model="isSearchModalActive" />

      <ResendActivationNotification :authenticated="login.authenticated" />

      <slot :authenticated="login.authenticated" :keypair="crypto.keypair" />

      <StatusBar v-if="login.authenticated" />
    </div>
  </div>
</template>
