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
import {
  mdiFolder,
  mdiFolderOpen,
  mdiFolderAccount,
  mdiFileDocumentOutline,
  mdiFileOutline,
  mdiChevronRight,
  mdiChevronDown
} from '@mdi/js'
import { isMarkdownFile } from '!/preview'
import * as meta from '!/storage/meta'
import { onFileTreeChange } from '!/storage/events'
import * as sharesApi from '!/shares/api'
import { SHARED_WITH_ME_DIR_ID, store as filesStore } from '!/storage'
import type { KeyPair } from 'types'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import AsideFileTreeNode from '@/components/ui/AsideFileTreeNode.vue'

const props = defineProps<{
  keypair: KeyPair
}>()

const files = filesStore()

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

function sortListing(items: AppFile[]): AppFile[] {
  return items.sort((a, b) => {
    if (a.mime === 'dir' && b.mime !== 'dir') return -1
    if (a.mime !== 'dir' && b.mime === 'dir') return 1
    return (a.name || '').localeCompare(b.name || '')
  })
}

async function fetchChildren(dirId?: string): Promise<AppFile[]> {
  const params = dirId ? { dir_id: dirId } : {}
  const response = await meta.find(params)

  const items: AppFile[] = []
  const privateKey = props.keypair.wrappingPrivate || (props.keypair.input as string)

  for (const item of response.children) {
    const decrypted = await meta.decrypt(item, privateKey)
    items.push({ ...item, ...decrypted } as AppFile)
  }

  return sortListing(items)
}

function toNode(file: AppFile): TreeNode {
  return { file, children: [], loaded: false, loading: false }
}

/**
 * Root node for the synthetic "Shared with me" folder. Pinned first in the
 * sidebar tree with the `mdiFolderAccount` icon. Unlike owned folders the
 * children are not fetched from `/api/storage` — they come from the
 * recipient-roots filter on `/api/shares/mine`, so the data source can't be
 * shared with `fetchChildren`. The node renders with `loaded: false` so the
 * expand affordance triggers `loadSharedRoots` on first click.
 */
function syntheticSharedNode(): TreeNode {
  const file: AppFile = {
    id: SHARED_WITH_ME_DIR_ID,
    user_id: '',
    is_owner: false,
    name: 'Shared with me',
    name_hash: '',
    mime: 'dir',
    chunks: 0,
    file_id: null,
    file_modified_at: 0,
    created_at: 0,
    is_new: false,
    editable: false,
    active_version: 1,
    encrypted_key: '',
    encrypted_name: '',
    cipher: ''
  } as AppFile
  return { file, children: [], loaded: false, loading: false }
}

async function hasIncomingShares(): Promise<boolean> {
  try {
    const page = await sharesApi.getSharesMine(1, 0)
    return page.total > 0 || page.items.length > 0
  } catch {
    return false
  }
}

/**
 * Translate the recipient's incoming-share roots into tree nodes. Each row
 * is the entry point into a shared file or folder — owned-by-someone-else
 * content the recipient can navigate into. Descendants stay server-side; the
 * tree shows the share roots only, same shape as the virtual folder in the
 * main file list.
 */
async function loadSharedRoots(): Promise<void> {
  const node = treeState.rootNodes.find((n) => n.file.id === SHARED_WITH_ME_DIR_ID)
  if (!node) return
  if (node.loaded || node.loading) return
  node.loading = true
  try {
    const page = await sharesApi.getSharesMine()
    const privateKey = props.keypair.wrappingPrivate || (props.keypair.input as string)
    const children: AppFile[] = []
    for (const row of page.items) {
      const base: AppFile = {
        id: row.file_id,
        user_id: row.owner_id,
        is_owner: false,
        name: row.file_id,
        name_hash: '',
        mime: row.mime,
        size: row.size ?? undefined,
        chunks: row.chunks ?? 0,
        chunks_stored: row.chunks_stored ?? undefined,
        finished_upload_at: row.finished_upload_at ?? undefined,
        file_id: SHARED_WITH_ME_DIR_ID,
        file_modified_at: row.created_at,
        created_at: row.created_at,
        is_new: false,
        editable: row.editable,
        active_version: 1,
        encrypted_key: row.encrypted_key,
        encrypted_name: row.encrypted_name,
        cipher: row.cipher,
        share_role: row.share_role,
        shared_by_email: row.shared_by_email ?? row.owner_email,
        owner_email: row.owner_email
      } as AppFile
      try {
        const decrypted = await meta.decrypt(
          {
            cipher: row.cipher,
            encrypted_key: row.encrypted_key,
            encrypted_name: row.encrypted_name
          },
          privateKey
        )
        children.push({ ...base, ...decrypted })
      } catch {
        children.push(base)
      }
    }
    children.sort((a, b) => {
      if (a.mime === 'dir' && b.mime !== 'dir') return -1
      if (a.mime !== 'dir' && b.mime === 'dir') return 1
      return (a.name || '').localeCompare(b.name || '')
    })
    node.children = children.map(toNode)
    node.loaded = true
  } finally {
    node.loading = false
  }
}

async function loadRoot() {
  rootLoading.value = true

  let items: AppFile[]
  let incomingShared: boolean

  // First load while the main view is fetching this exact listing (files
  // route, at root): seed from its rows and its shares probe instead of
  // racing it with a duplicate request and a second decrypt pass. Every
  // other route and later reloads keep their own fetch — on other routes
  // the main view never lists root, so there is nothing to wait for.
  if (!treeState.loaded && route.name === 'files' && !activeFolderId.value) {
    const rows = await files.firstRootListing()
    incomingShared = rows.some((row) => row.id === SHARED_WITH_ME_DIR_ID)
    items = sortListing(rows.filter((row) => row.id !== SHARED_WITH_ME_DIR_ID))
  } else {
    ;[items, incomingShared] = await Promise.all([fetchChildren(undefined), hasIncomingShares()])
  }
  const nodes = items.map(toNode)
  treeState.rootNodes = incomingShared ? [syntheticSharedNode(), ...nodes] : nodes
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
  // The synthetic root is a client-only marker; the server has no row
  // for it and would 400 on the parsed-id branch. Auto-expand for the
  // virtual folder is handled by `loadSharedRoots` below.
  if (folderId === SHARED_WITH_ME_DIR_ID) {
    treeState.expanded.add(folderId)
    await loadSharedRoots()
    return
  }
  const response = await meta.find({ dir_id: folderId })
  const privateKey = props.keypair.wrappingPrivate || (props.keypair.input as string)

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

  if (id === SHARED_WITH_ME_DIR_ID) {
    if (treeState.expanded.has(id)) {
      treeState.expanded.delete(id)
    } else {
      treeState.expanded.add(id)
      await loadSharedRoots()
    }
    router.push({ name: 'files', params: { file_id: id } })
    return
  }

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
  if (file.id === SHARED_WITH_ME_DIR_ID) return mdiFolderAccount
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
    const synthetic = treeState.rootNodes.find(
      (n) => n.file.id === SHARED_WITH_ME_DIR_ID
    )
    const merged = mergeChildren(treeState.rootNodes, items)
    treeState.rootNodes = synthetic ? [synthetic, ...merged] : merged
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
          v-if="node.file.id === SHARED_WITH_ME_DIR_ID"
          :title="node.file.name"
          class="flex items-center gap-1 px-2 py-1 cursor-pointer transition-colors duration-150"
          :class="
            node.file.id === activeFolderId
              ? 'bg-orangy-500/15 text-orangy-300'
              : 'text-brownish-300 hover:bg-brownish-700/50 hover:text-brownish-100'
          "
          data-testid="aside-tree-shared-with-me"
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

        <li
          v-else-if="node.file.mime === 'dir'"
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
            <li
              v-if="!node.children.length"
              class="py-1 text-brownish-500 italic"
              style="padding-left: 24px"
              :data-testid="
                node.file.id === SHARED_WITH_ME_DIR_ID
                  ? 'aside-tree-shared-with-me-empty'
                  : undefined
              "
            >
              {{ node.file.id === SHARED_WITH_ME_DIR_ID ? 'No incoming shares yet' : 'Empty' }}
            </li>
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
