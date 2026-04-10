<script setup lang="ts">
import { mdiFolder, mdiFolderOpen, mdiFileDocumentOutline, mdiFileOutline, mdiChevronRight, mdiChevronDown } from '@mdi/js'
import { isMarkdownFile } from '!/preview'
import * as meta from '!/storage/meta'
import type { KeyPair, AppFile } from 'types'
import BaseIcon from '@/components/ui/BaseIcon.vue'

interface TreeNode {
  file: AppFile
  children: TreeNode[]
  loaded: boolean
  loading: boolean
}

const props = defineProps<{
  node: TreeNode
  depth: number
  expanded: Set<string>
  activeFileId?: string
  activeFolderId?: string
  keypair: KeyPair
}>()

const emit = defineEmits<{
  'toggle-folder': [node: TreeNode]
  'file-click': [file: AppFile]
}>()

const padLeft = `padding-left: ${props.depth * 16 + 8}px`

function iconFor(file: AppFile): string {
  if (file.mime === 'dir') {
    return props.expanded.has(file.id) ? mdiFolderOpen : mdiFolder
  }
  if (isMarkdownFile(file)) return mdiFileDocumentOutline
  return mdiFileOutline
}

async function fetchAndExpand(node: TreeNode) {
  if (!node.loaded) {
    node.loading = true
    const response = await meta.find({ dir_id: node.file.id })
    const privateKey = props.keypair.input as string

    const items: AppFile[] = []
    for (const item of response.children) {
      const decrypted = await meta.decrypt(item, privateKey)
      items.push({ ...item, ...decrypted } as AppFile)
    }

    items.sort((a, b) => {
      if (a.mime === 'dir' && b.mime !== 'dir') return -1
      if (a.mime !== 'dir' && b.mime === 'dir') return 1
      return (a.name || '').localeCompare(b.name || '')
    })

    node.children = items.map((f) => ({ file: f, children: [], loaded: false, loading: false }))
    node.loaded = true
    node.loading = false
  }

  emit('toggle-folder', node)
}
</script>

<template>
  <li
    v-if="node.file.mime === 'dir'"
    class="flex items-center gap-1 py-1 cursor-pointer transition-colors duration-150"
    :class="
      node.file.id === activeFolderId
        ? 'bg-orangy-500/15 text-orangy-300'
        : 'text-brownish-300 hover:bg-brownish-700/50 hover:text-brownish-100'
    "
    :style="padLeft"
    @click="fetchAndExpand(node)"
  >
    <BaseIcon
      :path="expanded.has(node.file.id) ? mdiChevronDown : mdiChevronRight"
      :size="12"
      class="flex-shrink-0 text-brownish-500"
    />
    <BaseIcon :path="iconFor(node.file)" :size="14" class="flex-shrink-0 text-orangy-400" />
    <span class="truncate">{{ node.file.name }}</span>
  </li>

  <template v-if="node.file.mime === 'dir' && expanded.has(node.file.id)">
    <li v-if="node.loading" class="py-1 text-brownish-500" :style="`padding-left: ${(depth + 1) * 16 + 24}px`">
      Loading...
    </li>
    <template v-else>
      <AsideFileTreeNode
        v-for="child in node.children"
        :key="child.file.id"
        :node="child"
        :depth="depth + 1"
        :expanded="expanded"
        :active-file-id="activeFileId"
        :active-folder-id="activeFolderId"
        :keypair="keypair"
        @toggle-folder="(n: TreeNode) => emit('toggle-folder', n)"
        @file-click="(f: AppFile) => emit('file-click', f)"
      />
      <li
        v-if="!node.children.length"
        class="py-1 text-brownish-500 italic"
        :style="`padding-left: ${(depth + 1) * 16 + 24}px`"
      >
        Empty
      </li>
    </template>
  </template>

  <li
    v-else-if="node.file.mime !== 'dir'"
    class="flex items-center gap-1 py-1 cursor-pointer transition-colors duration-150"
    :class="
      node.file.id === activeFileId
        ? 'bg-orangy-500/15 text-orangy-300'
        : 'text-brownish-300 hover:bg-brownish-700/50 hover:text-brownish-100'
    "
    :style="padLeft"
    @click="emit('file-click', node.file)"
  >
    <span class="w-3 flex-shrink-0" />
    <BaseIcon :path="iconFor(node.file)" :size="14" class="flex-shrink-0" />
    <span class="truncate">{{ node.file.name }}</span>
  </li>
</template>
