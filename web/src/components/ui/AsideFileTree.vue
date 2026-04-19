<script lang="ts">
import { reactive } from 'vue'
import type { AppFile } from 'types'

export interface TreeNode {
  file: AppFile
  children: TreeNode[]
  loaded: boolean
  loading: boolean
}

// Module-level state — survives component unmount/remount
const treeState = reactive({
  rootNodes: [] as TreeNode[],
  expanded: new Set<string>(),
  loaded: false,
  userFingerprint: undefined as string | undefined
})
</script>

<script setup lang="ts">
import { computed, ref, watch, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { mdiFolder, mdiFolderOpen, mdiFileDocumentOutline, mdiFileOutline, mdiChevronRight, mdiChevronDown } from '@mdi/js'
import { isMarkdownFile } from '!/preview'
import * as meta from '!/storage/meta'
import { onFileTreeChange } from '!/storage/events'
import type { KeyPair } from 'types'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import AsideFileTreeNode from '@/components/ui/AsideFileTreeNode.vue'

const props = defineProps<{
  keypair: KeyPair
}>()

const route = useRoute()
const router = useRouter()

const rootLoading = ref(false)

const activeFileId = computed(() => {
  const id = route.params.id || route.params.file_id
  return Array.isArray(id) ? id[0] : (id as string | undefined)
})

const activeFolderId = computed(() => {
  const id = route.params.file_id
  return Array.isArray(id) ? id[0] : (id as string | undefined)
})

async function fetchChildren(dirId?: string): Promise<AppFile[]> {
  const params = dirId ? { dir_id: dirId } : {}
  const response = await meta.find(params)

  const items: AppFile[] = []
  const privateKey = props.keypair.input as string

  for (const item of response.children) {
    const decrypted = await meta.decrypt(item, privateKey)
    items.push({ ...item, ...decrypted } as AppFile)
  }

  items.sort((a, b) => {
    if (a.mime === 'dir' && b.mime !== 'dir') return -1
    if (a.mime !== 'dir' && b.mime === 'dir') return 1
    return (a.name || '').localeCompare(b.name || '')
  })

  return items
}

function toNode(file: AppFile): TreeNode {
  return { file, children: [], loaded: false, loading: false }
}

async function loadRoot() {
  rootLoading.value = true
  const items = await fetchChildren(undefined)
  treeState.rootNodes = items.map(toNode)
  treeState.loaded = true
  treeState.userFingerprint = props.keypair.fingerprint || undefined
  rootLoading.value = false

  // Auto-expand tree to match the current route
  const folderId = activeFolderId.value
  if (folderId) {
    await expandToFolder(folderId)
  }
}

async function expandToFolder(folderId: string) {
  // Fetch the directory to get its parent chain
  const response = await meta.find({ dir_id: folderId })
  const privateKey = props.keypair.input as string

  const parents: AppFile[] = []
  for (const item of response.parents || []) {
    const decrypted = await meta.decrypt(item, privateKey)
    parents.push({ ...item, ...decrypted } as AppFile)
  }

  // parents come in root-first order from the backend
  // Expand each ancestor, loading children along the way
  for (const parent of parents) {
    let node = findNode(treeState.rootNodes, parent.id)
    if (!node) continue

    if (!node.loaded) {
      node.loading = true
      const children = await fetchChildren(parent.id)
      node.children = children.map(toNode)
      node.loaded = true
      node.loading = false
    }

    treeState.expanded.add(parent.id)
  }

  // Also expand the target folder itself
  let targetNode = findNode(treeState.rootNodes, folderId)
  if (targetNode) {
    if (!targetNode.loaded) {
      targetNode.loading = true
      const children = await fetchChildren(folderId)
      targetNode.children = children.map(toNode)
      targetNode.loaded = true
      targetNode.loading = false
    }
    treeState.expanded.add(folderId)
  }
}

async function toggleFolder(node: TreeNode) {
  const id = node.file.id

  if (treeState.expanded.has(id)) {
    treeState.expanded.delete(id)
  } else {
    treeState.expanded.add(id)
    if (!node.loaded) {
      node.loading = true
      const items = await fetchChildren(id)
      node.children = items.map(toNode)
      node.loaded = true
      node.loading = false
    }
  }

  router.push({ name: 'files', params: { file_id: id } })
}

function onFileClick(file: AppFile) {
  if (isMarkdownFile(file)) {
    router.push({ name: 'notes', params: { id: file.id } })
  } else {
    router.push({ name: 'file-preview', params: { id: file.id } })
  }
}

function iconFor(file: AppFile): string {
  if (file.mime === 'dir') {
    return treeState.expanded.has(file.id) ? mdiFolderOpen : mdiFolder
  }
  if (isMarkdownFile(file)) return mdiFileDocumentOutline
  return mdiFileOutline
}

function findNode(nodes: TreeNode[], id: string): TreeNode | undefined {
  for (const node of nodes) {
    if (node.file.id === id) return node
    const found = findNode(node.children, id)
    if (found) return found
  }
}

function mergeChildren(existing: TreeNode[], fresh: AppFile[]): TreeNode[] {
  const byId = new Map(existing.map((n) => [n.file.id, n]))
  return fresh.map((f) => {
    const prev = byId.get(f.id)
    return prev ? { ...prev, file: f } : toNode(f)
  })
}

async function refreshFolder(folderId?: string) {
  const items = await fetchChildren(folderId)

  if (!folderId) {
    treeState.rootNodes = mergeChildren(treeState.rootNodes, items)
    return
  }

  const node = findNode(treeState.rootNodes, folderId)
  if (node && node.loaded) {
    node.children = mergeChildren(node.children, items)
  }
}

const unsubscribe = onFileTreeChange(async (event) => {
  const folders = new Set<string | undefined>()
  folders.add(event.folderId)
  if (event.targetFolderId !== undefined) folders.add(event.targetFolderId)

  for (const folderId of folders) {
    try {
      await refreshFolder(folderId)
    } catch (err) {
      console.error('Failed to refresh folder in tree:', folderId, err)
    }
  }
})

onUnmounted(() => unsubscribe())

watch(
  () => props.keypair,
  (kp) => {
    const fingerprint = kp.fingerprint || undefined
    if (treeState.loaded && treeState.userFingerprint === fingerprint) return
    loadRoot()
  },
  { immediate: true }
)
</script>

<template>
  <div class="text-xs ml-6 mr-2 my-1 rounded-lg bg-brownish-800/40 overflow-hidden">
    <div v-if="rootLoading" class="flex items-center justify-center py-4 text-brownish-500">
      Loading...
    </div>
    <ul v-else class="flex flex-col py-0.5">
      <template v-for="node in treeState.rootNodes" :key="node.file.id">
        <li
          v-if="node.file.mime === 'dir'"
          :title="node.file.name"
          class="flex items-center gap-1 px-2 py-1 cursor-pointer transition-colors duration-150"
          :class="
            node.file.id === activeFolderId
              ? 'bg-orangy-500/15 text-orangy-300'
              : 'text-brownish-300 hover:bg-brownish-700/50 hover:text-brownish-100'
          "
          @click="toggleFolder(node)"
        >
          <BaseIcon
            :path="treeState.expanded.has(node.file.id) ? mdiChevronDown : mdiChevronRight"
            :size="12"
            class="flex-shrink-0 text-brownish-500"
          />
          <BaseIcon :path="iconFor(node.file)" :size="14" class="flex-shrink-0 text-orangy-400" />
          <span class="truncate">{{ node.file.name }}</span>
        </li>

        <template v-if="node.file.mime === 'dir' && treeState.expanded.has(node.file.id)">
          <li v-if="node.loading" class="py-1 text-brownish-500" style="padding-left: 24px">Loading...</li>
          <template v-else>
            <AsideFileTreeNode
              v-for="child in node.children"
              :key="child.file.id"
              :node="child"
              :depth="1"
              :expanded="treeState.expanded"
              :active-file-id="activeFileId"
              :active-folder-id="activeFolderId"
              :keypair="keypair"
              @toggle-folder="toggleFolder"
              @file-click="onFileClick"
            />
            <li v-if="!node.children.length" class="py-1 text-brownish-500 italic" style="padding-left: 24px">Empty</li>
          </template>
        </template>

        <li
          v-else-if="node.file.mime !== 'dir'"
          :title="node.file.name"
          class="flex items-center gap-1 px-2 py-1 cursor-pointer transition-colors duration-150"
          :class="
            node.file.id === activeFileId
              ? 'bg-orangy-500/15 text-orangy-300'
              : 'text-brownish-300 hover:bg-brownish-700/50 hover:text-brownish-100'
          "
          @click="onFileClick(node.file)"
        >
          <span class="w-3 flex-shrink-0" />
          <BaseIcon :path="iconFor(node.file)" :size="14" class="flex-shrink-0" />
          <span class="truncate">{{ node.file.name }}</span>
        </li>
      </template>

      <li v-if="!treeState.rootNodes.length && !rootLoading" class="px-3 py-4 text-center text-brownish-500">
        No files here yet
      </li>
    </ul>
  </div>
</template>
