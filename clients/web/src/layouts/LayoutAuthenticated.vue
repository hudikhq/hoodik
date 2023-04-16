<script setup lang="ts">
import { mdiForwardburger, mdiBackburger, mdiMenu } from '@mdi/js'
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import menuAside from '@/menuAside'
import menuNavBar, { type NavBarItem } from '@/menuNavBar'
import { store as style } from '@/stores/style'
import { store as login } from '@/stores/auth/login'
import { store as crypto } from '@/stores/crypto'
import { ensureAuthenticated } from '@/stores/auth'
import BaseIcon from '@/components/ui/BaseIcon.vue'
// import FormControl from '@/components/ui/FormControl.vue'
import NavBar from '@/components/ui/NavBar.vue'
import NavBarItemPlain from '@/components/ui/NavBarItemPlain.vue'
import AsideMenu from '@/components/ui/AsideMenu.vue'
import FooterBar from '@/components/ui/FooterBar.vue'
import UploadAction from '@/components/actions/UploadAction.vue'

const styleStore = style()
const router = useRouter()
const loginStore = login()
const cryptoStore = crypto()

ensureAuthenticated(router, loginStore, cryptoStore)

const layoutAsidePadding = 'xl:pl-60'

const isAsideMobileExpanded = ref(false)
const isAsideLgActive = ref(false)

router.beforeEach(() => {
  isAsideMobileExpanded.value = false
  isAsideLgActive.value = false
})

const menuClick = (event: Event, item: NavBarItem) => {
  if (item.isToggleLightDark) {
    styleStore.setDarkMode()
  }

  if (item.isLogout) {
    //
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
      class="pt-14 min-h-screen w-screen transition-position lg:w-auto bg-gray-50 dark:bg-slate-800 dark:text-slate-100"
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
        <!-- <NavBarItemPlain use-margin>
          <FormControl placeholder="Search (ctrl+k)" ctrl-k-focus transparent borderless />
        </NavBarItemPlain> -->
      </NavBar>
      <AsideMenu
        :is-aside-mobile-expanded="isAsideMobileExpanded"
        :is-aside-lg-active="isAsideLgActive"
        :menu="menuAside"
        @menu-click="menuClick"
        @aside-lg-close-click="isAsideLgActive = false"
      />
      <slot />
      <FooterBar> </FooterBar>
      <UploadAction />
    </div>
  </div>
</template>
