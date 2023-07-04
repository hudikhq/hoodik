<script lang="ts" setup>
import type { AppFile, FilesStore, KeyPair } from 'types'
import { computed, ref, watch } from 'vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import { mdiChevronDown, mdiChevronUp, mdiFolderMove } from '@mdi/js'
import BaseButtonConfirm from '@/components/ui/BaseButtonConfirm.vue'
import DirectoryTree from '@/components/files/browser/DirectoryTree.vue'

const props = defineProps<{
  parent?: AppFile
  load?: boolean
  keypair: KeyPair
  Storage: FilesStore
}>()

const emits = defineEmits<{
  (event: 'select', value?: AppFile): void
}>()

const select = (file?: AppFile) => {
  emits('select', file)
}

const disabled = computed(() => {
  if (!props.parent) return false
  return props.Storage.selected.some((item) => item.id === props.parent?.id)
})

const items = ref<AppFile[]>([])
const opened = ref(false)

watch(
  opened,
  async (value) => {
    if (value) {
      items.value = await props.Storage.directories(props.keypair, props.parent?.id)
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
</script>
<template>
  <ul>
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
        @select="select"
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
