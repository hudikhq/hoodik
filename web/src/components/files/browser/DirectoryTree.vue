<script lang="ts" setup>
import type { AppFile, FilesStore, KeyPair } from 'types'
import { computed, ref, watch } from 'vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import {
  mdiChevronDown,
  mdiChevronUp,
  mdiChevronRight,
  mdiFolderMove,
  mdiFolder,
  mdiFolderOpen,
  mdiMonitor
} from '@mdi/js'
import BaseButtonConfirm from '@/components/ui/BaseButtonConfirm.vue'
import DirectoryTree from '@/components/files/browser/DirectoryTree.vue'
import { RouterLink, useRoute } from 'vue-router'
import { store as storageStore } from '!/storage'
import { store as style } from '!/style'

const props = withDefaults(
  defineProps<{
    parent?: AppFile
    load?: boolean
    keypair: KeyPair
    Storage?: FilesStore
    mode?: 'select' | 'navigate'
    activeFolderId?: string
    expandedIds?: Set<string>
    depth?: number
  }>(),
  {
    mode: 'select',
    depth: 0
  }
)

const emits = defineEmits<{
  (event: 'select', value?: AppFile): void
}>()

const styleStore = style()
const route = props.mode === 'navigate' ? useRoute() : null

// In navigate mode, use the store directly; in select mode, use the prop
const storageStoreInstance = props.mode === 'navigate' ? storageStore() : null

const getStorage = (): FilesStore => {
  if (props.Storage) return props.Storage
  if (storageStoreInstance) return storageStoreInstance
  throw new Error('DirectoryTree requires either Storage prop or navigate mode')
}

const select = (file?: AppFile) => {
  emits('select', file)
}

const disabled = computed(() => {
  if (props.mode === 'navigate') return false
  if (!props.parent) return false
  return getStorage().selected.some((item) => item.id === props.parent?.id)
})

const isRoot = computed(() => props.mode === 'navigate' && !props.parent)

const isActive = computed(() => {
  if (props.mode !== 'navigate') return false
  if (!props.parent) return isOnFilesRoute.value && !props.activeFolderId
  return props.parent.id === props.activeFolderId
})

const isOnFilesRoute = computed(() => {
  if (!route) return false
  return route.name === 'files'
})

const items = ref<AppFile[]>([])
const opened = ref(false)

watch(
  opened,
  async (value) => {
    if (value) {
      items.value = await getStorage().directories(props.keypair, props.parent?.id)
    } else {
      items.value = []
    }
  },
  { immediate: true }
)

watch(
  () => props.load,
  async (value) => {
    if (value) {
      opened.value = true
    }
  },
  { immediate: true }
)

// Auto-open/close root based on files route
watch(
  isOnFilesRoute,
  (onFiles) => {
    if (!isRoot.value) return
    if (onFiles && !opened.value) {
      opened.value = true
    } else if (!onFiles && opened.value) {
      opened.value = false
    }
  },
  { immediate: true }
)

// Auto-expand when this folder is in the expandedIds set (navigate mode)
watch(
  () => props.expandedIds,
  (ids) => {
    if (props.mode !== 'navigate' || !ids || !props.parent) return
    if (ids.has(props.parent.id) && !opened.value) {
      opened.value = true
    }
  },
  { immediate: true, deep: true }
)
</script>
<template>
  <!-- Select mode (move modal) - original layout -->
  <ul v-if="mode === 'select'">
    <li
      class="w-full border-t-[1px] border-brownish-800 p-1"
      :class="{
        'pl-4': parent
      }"
    >
      <div class="flex flex-shrink">
        <div class="w-full cursor-pointer prevent-select" @click="opened = !opened">
          <BaseIcon :path="opened ? mdiChevronDown : mdiChevronUp" size="20" w="w-6" h="h-6" />
          {{ parent?.name || 'Root' }}
        </div>
        <div class="text-right whitespace-nowrap">
          <BaseButtonConfirm
            :icon="mdiFolderMove"
            @confirm="select(parent)"
            label="Move"
            confirm-label="Confirm"
            :xs="true"
            :disabled="disabled"
          />
        </div>
      </div>
    </li>
    <li class="pl-4" v-if="opened">
      <DirectoryTree
        v-for="item in items"
        :key="item.id"
        :keypair="keypair"
        :Storage="Storage"
        :parent="item"
        mode="select"
        @select="select"
      />
    </li>
  </ul>

  <!-- Navigate mode (sidebar) — root node styled as menu item -->
  <ul v-else-if="isRoot">
    <li>
      <div
        class="flex cursor-pointer py-3"
        :class="[
          isActive
            ? styleStore.asideMenuItemActiveStyle
            : `${styleStore.asideMenuItemStyle} dark:text-brownish-300 dark:hover:text-white`
        ]"
        @click="opened = !opened"
      >
        <BaseIcon
          :path="mdiMonitor"
          class="flex-none"
          :class="[isActive ? styleStore.asideMenuItemActiveStyle : '']"
          w="w-16"
          :size="18"
        />
        <RouterLink
          :to="{ name: 'files' }"
          class="grow text-ellipsis line-clamp-1"
          @click.stop
        >
          <span :class="[isActive ? styleStore.asideMenuItemActiveStyle : '']">My Files</span>
        </RouterLink>
      </div>
    </li>
    <li v-if="opened && items.length > 0">
      <DirectoryTree
        v-for="item in items"
        :key="item.id"
        :keypair="keypair"
        :parent="item"
        mode="navigate"
        :active-folder-id="activeFolderId"
        :expanded-ids="expandedIds"
        :depth="1"
      />
    </li>
  </ul>

  <!-- Navigate mode (sidebar) — child folder nodes -->
  <ul v-else class="text-sm">
    <li>
      <div
        class="flex items-center cursor-pointer py-1.5 px-2 rounded transition-colors"
        :style="{ paddingLeft: (depth * 16 + 8) + 'px' }"
        :class="[
          isActive
            ? styleStore.asideMenuItemActiveStyle
            : `${styleStore.asideMenuItemStyle} dark:text-brownish-300 dark:hover:text-white`
        ]"
      >
        <span
          class="flex-none w-5 flex items-center justify-center"
          @click.stop="opened = !opened"
        >
          <BaseIcon
            v-if="items.length > 0 || !opened"
            :path="opened ? mdiChevronDown : mdiChevronRight"
            size="14"
            w="w-4"
            h="h-4"
          />
        </span>
        <BaseIcon
          :path="opened ? mdiFolderOpen : mdiFolder"
          size="16"
          w="w-5"
          h="h-5"
          class="flex-none mr-1.5"
        />
        <RouterLink
          :to="{ name: 'files', params: { file_id: parent?.id } }"
          class="grow text-ellipsis line-clamp-1 prevent-select"
          @click.stop
        >
          {{ parent?.name }}
        </RouterLink>
      </div>
    </li>
    <li v-if="opened && items.length > 0">
      <DirectoryTree
        v-for="item in items"
        :key="item.id"
        :keypair="keypair"
        :parent="item"
        mode="navigate"
        :active-folder-id="activeFolderId"
        :expanded-ids="expandedIds"
        :depth="(depth || 0) + 1"
      />
    </li>
  </ul>
</template>

<style lang="css">
.prevent-select {
  -webkit-user-select: none;
  -ms-user-select: none;
  user-select: none;
}
</style>
