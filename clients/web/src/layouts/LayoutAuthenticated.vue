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
// import FormControl from '@/components/ui/FormControl.vue'
import NavBar from '@/components/ui/NavBar.vue'
import NavBarItemPlain from '@/components/ui/NavBarItemPlain.vue'
import AsideMenu from '@/components/ui/AsideMenu.vue'
import FooterBar from '@/components/ui/FooterBar.vue'
import IOStatus from '@/components/files/IOStatus.vue'
import CreateDirectoryModal from '@/components/actions/CreateDirectoryModal.vue'
import { store as storageStore } from '@/stores/storage'
import { store as cryptoStore } from '@/stores/crypto'
import { store as uploadStore } from '@/stores/storage/upload'

const upload = uploadStore()
const crypto = cryptoStore()
const storage = storageStore()

const styleStore = style()
const router = useRouter()
const loginStore = login()

await ensureAuthenticated(router, loginStore, crypto)

const layoutAsidePadding = 'xl:pl-60'

const isModalCreateDirectory = ref(false)
const isAsideMobileExpanded = ref(false)
const isAsideLgActive = ref(false)
const fileInput = ref<HTMLInputElement>()

const add = async () => {
  if (fileInput.value && fileInput.value?.files?.length) {
    for (let i = 0; i < fileInput.value?.files?.length; i++) {
      await upload.push(crypto.keypair, fileInput.value?.files?.[i], storage.dir?.id || undefined)
    }
  }

  if (!upload.active) {
    await upload.start(storage, crypto.keypair)
  }
}

router.beforeEach(() => {
  isAsideMobileExpanded.value = false
  isAsideLgActive.value = false
})

const menuClick = (event: Event, item: NavBarItem) => {
  if (item.isToggleLightDark) {
    styleStore.setDarkMode()
  }

  if (item.isUpload && fileInput.value) {
    fileInput.value.click()
  }

  if (item.isCreateDirectory) {
    isModalCreateDirectory.value = true
  }
}

const cancel = () => {
  isModalCreateDirectory.value = false
}
</script>

<template>
  <input style="display: none" type="file" ref="fileInput" multiple @change="add" />

  <div
    :class="{
      dark: styleStore.darkMode,
      'overflow-hidden lg:overflow-visible': isAsideMobileExpanded
    }"
  >
    <div
      :class="[layoutAsidePadding, { 'ml-60 lg:ml-0': isAsideMobileExpanded }]"
      class="pt-5 min-h-screen w-screen transition-position lg:w-auto bg-gray-50 dark:bg-slate-800 dark:text-slate-100"
    >
      <IOStatus />
      <CreateDirectoryModal v-model="isModalCreateDirectory" @cancel="cancel" />
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
      <slot :authenticated="loginStore.authenticated" :keypair="crypto.keypair" />
      <FooterBar> </FooterBar>
    </div>
  </div>
</template>
