import * as meta from './meta'
import * as queue from '../queue'
import * as upload from './upload'
import * as download from './download'
import { emitFileTreeChange } from './events'
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import * as cryptfns from '../cryptfns'
import { utcStringFromLocal, uuidv4 } from '..'
import { useStorage } from '@vueuse/core'

import type {
  AppFile,
  CreateFile,
  FileResponse,
  Parameters,
  EncryptedAppFile,
  IncomingShare,
  KeyPair,
  StorageStatsResponse
} from 'types'

export { meta, upload, download, queue }

/**
 * Synthetic directory id for the "Shared with me" virtual folder rendered
 * at the root of `/files`. Recipient-side incoming shares are mapped into
 * rows under this id so the file browser can navigate them like any
 * regular folder. The server never sees this id — the storage store
 * branches on it before any network call.
 */
export const SHARED_WITH_ME_DIR_ID = '__shared_with_me__'

/**
 * Whether the caller may write into a folder shared with them. Editors and
 * Co-owners can; readers cannot. A file the caller owns has no `share_role`
 * and is never a "shared with me" destination, so it falls through to false.
 */
export function canWriteToShared(file: AppFile): boolean {
  return file.share_role === 'editor' || file.share_role === 'co-owner'
}

/**
 * Run sort operations on the given items by the given parameter
 * The results are always in ASC, if you need DESC, just reverse the array
 */
function innerSort(items: AppFile[], parameter: string): AppFile[] {
  return items.sort((a, b) => {
    // @ts-ignore
    const aValue = a[parameter] || ''
    // @ts-ignore
    const bValue = b[parameter] || ''

    if (typeof aValue === 'number' && typeof bValue === 'number') {
      return aValue - bValue
    }

    return aValue.localeCompare(bValue)
  })
}

export const store = defineStore('files', () => {
  /**
   * Are we loading the files?
   */
  const loading = ref(false)

  /**
   * Title of the currently selected directory (or root)
   */
  const title = computed<string>((): string => {
    return dir.value?.name || 'Root'
  })

  /**
   * Lookup parameters
   */
  const parameters = computed<Parameters>(() => {
    const parameters: Parameters = {}

    if (dir.value) {
      parameters['dir_id'] = dir.value?.id
    }

    return parameters
  })

  /**
   * All the items regardless of the current directory
   */
  const _items = ref<AppFile[]>([])

  /**
   * Persistent storage of the sort options
   */
  const sort = useStorage<{ [key: string]: string }>('dir-sort', {})

  /**
   * Currently selected directory id
   */
  const fileId = ref<string | undefined>()

  /**
   * Last error message that happened when trying to
   * fetch the files from the backend.
   */
  const error = ref<string | null>(null)

  /**
   * Storage stats for the current user
   */
  const stats = ref<StorageStatsResponse>()

  /**
   * Selected files
   */
  const selected = ref<AppFile[]>([])

  /**
   * Current dir sort options
   */
  const sortOptions = computed<{ parameter: string; order: string }>(() => {
    const [parameter, order] = getSort(fileId.value || 'root').split('|')

    return {
      parameter,
      order
    }
  })

  /**
   * Currently selected directory
   */
  const dir = computed<AppFile | null>(() => {
    return _items.value.find((item) => item.mime === 'dir' && item.id === fileId.value) || null
  })

  /**
   * All the parent directories
   */
  const parents = computed<AppFile[]>(() => {
    const p: AppFile[] = []

    const f = (id: string | undefined) => {
      const i = _items.value.find((item) => item.id === id)

      if (i) {
        p.push(i)
      }

      if (i?.file_id) {
        f(i.file_id)
      }
    }

    f(fileId.value)

    return p.reverse()
  })

  /**
   * Items filtered for the given directory and sorted by the sort options.
   *
   * The synthetic "Shared with me" entry is always pinned at index 0 of
   * the root listing regardless of the active sort — it's an injected
   * affordance, not user content, so the sort field shouldn't bury it
   * between owned folders that happen to alphabetize earlier.
   */
  const items = computed<AppFile[]>(() => {
    const [parameter, order] = getSort(fileId.value || 'root').split('|')

    const directories = innerSort(
      _items.value.filter((item) => {
        if (item.mime !== 'dir') {
          return false
        }

        if (item.id === SHARED_WITH_ME_DIR_ID) {
          return false
        }

        if (fileId.value) {
          return item.file_id === fileId.value
        }

        return item.file_id === null
      }),
      parameter
    )

    const files = innerSort(
      _items.value.filter((item) => {
        if (item.mime === 'dir') {
          return false
        }

        if (fileId.value) {
          return item.file_id === fileId.value
        }

        return item.file_id === null
      }),
      parameter
    )

    const ordered =
      order === 'desc'
        ? [...directories.reverse(), ...files.reverse()]
        : [...directories, ...files]

    if (!fileId.value) {
      const synthetic = _items.value.find((item) => item.id === SHARED_WITH_ME_DIR_ID)
      if (synthetic) return [synthetic, ...ordered]
    }

    return ordered
  })

  /**
   * Load the storage stats
   */
  async function loadStats(): Promise<void> {
    stats.value = await meta.stats()
  }

  /**
   * Return all the directories for the current directory
   */
  async function directories(kp: KeyPair, dir_id: string | undefined): Promise<AppFile[]> {
    const query = {
      ...parameters.value,
      dir_id,
      dirs_only: true,
      is_owner: true
    }

    const response = await meta.find(query)

    const children = response.children || []

    return Promise.all(children.map(async (item) => decryptItem(item, kp)))
  }

  /**
   * Folder roots shared with the caller, restricted to the ones they can
   * write to (editor / co-owner). Feeds the "Shared with me" branch of the
   * move-target picker: a reader-only folder is not a valid move
   * destination — the server would reject the move — so it never appears.
   */
  async function sharedRoots(kp: KeyPair): Promise<AppFile[]> {
    try {
      const rows = await findShared(kp)
      return rows.filter((row) => row.mime === 'dir' && canWriteToShared(row))
    } catch {
      // A server without sharing (or with it disabled) 404/503s the shares
      // endpoint. Soft-fail to no shared destinations so the move picker
      // still offers the caller's own drive.
      return []
    }
  }

  /**
   * Subdirectories of a shared folder, listed recipient-aware: unlike
   * `directories`, this omits the `is_owner` filter so the server returns
   * the child rows the caller has a key for inside someone else's folder
   * (the same listing the file browser uses to navigate into a share).
   * Restricted to writable children for the same reason as `sharedRoots`.
   */
  async function sharedDirectories(kp: KeyPair, dir_id: string): Promise<AppFile[]> {
    const response = await meta.find({ ...parameters.value, dir_id, dirs_only: true })
    const children = response.children || []
    const decrypted = await Promise.all(children.map((item) => decryptItem(item, kp)))
    return decrypted.filter((item) => canWriteToShared(item))
  }

  /**
   * Head over to backend and do a lookup for the current directory.
   *
   * Two synthetic branches sit on top of the regular `/api/storage` call:
   * - `parentId === SHARED_WITH_ME_DIR_ID` loads incoming shares via
   *   `findShared` and never hits the storage endpoint.
   * - `parentId === undefined` (root) also fans out a fetch of incoming
   *   shares so the virtual folder can be injected at the root level
   *   when the recipient has at least one share.
   */
  async function find(
    kp: KeyPair,
    parentId: string | undefined,
    showLoading = true
  ): Promise<void> {
    error.value = null

    // Selection is scoped to the currently visible list. Holding it across
    // navigation lets a row that's gone from the new view stay flagged for
    // bulk actions, surfacing as a phantom checked checkbox when the row's
    // location is reset (e.g. a shared folder that visited inside
    // `__shared_with_me__` then dropped back to the recipient's own root).
    if (parentId !== fileId.value) {
      selected.value = []
    }

    fileId.value = parentId

    if (parentId === SHARED_WITH_ME_DIR_ID) {
      loading.value = showLoading
      try {
        const rows = await findShared(kp)
        await ensureSharedWithMeRoot(rows.length > 0)
        // Drop stale rows that were revoked between renders; the
        // synthetic folder is the single source of truth for what the
        // recipient currently has access to.
        const nextIds = new Set(rows.map((row) => row.id))
        _items.value = _items.value.filter(
          (item) => item.file_id !== SHARED_WITH_ME_DIR_ID || nextIds.has(item.id)
        )
        rows.forEach((row) => upsertItem(row))
      } catch (e) {
        error.value = `Seems like we are having some kind of problem with getting the files: ${
          (e as Error).message
        }`
      } finally {
        loading.value = false
      }
      return
    }

    let query = parameters.value
    if (parentId !== undefined && parentId !== null) {
      query = { ...parameters.value, dir_id: parentId }
    } else {
      delete query.dir_id
    }

    let response: FileResponse = { children: [], parents: [] }
    loading.value = showLoading

    // We wrap this here so we can somewhat support failing network
    // connection and use the files we have in the store.
    try {
      response = await meta.find(query)
    } catch (e) {
      error.value = `Seems like we are having some kind of problem with getting the files: ${
        (e as Error).message
      }`
    }

    response.parents?.forEach(async (item) => {
      await replaceItem(item, kp)
    })

    response.children?.forEach(async (item) => {
      await replaceItem(item, kp)
    })

    if (parentId === undefined) {
      try {
        // Dynamic import keeps the shares wire layer out of the boot
        // bundle — only fetched when the file browser hits the root
        // listing path, not eagerly at every page load.
        const sharesApi = await import('../shares/api')
        const page = await sharesApi.getSharesMine(1, 0)
        await ensureSharedWithMeRoot(page.total > 0 || page.items.length > 0)
      } catch {
        // Soft-fail: the synthetic folder won't appear this render but
        // will on the next navigation. We don't want a 5xx on the
        // shares endpoint to block the regular file listing.
      }
    }

    loading.value = false
  }

  /**
   * Map the recipient-side incoming-share list into `AppFile` rows that
   * live inside the synthetic `SHARED_WITH_ME_DIR_ID` folder. Each row's
   * `encrypted_name` is decrypted with the caller's private key via the
   * same `meta.decrypt` helper used everywhere else, and `shared_by_email`
   * / `owner_email` are surfaced so the row component can render the
   * "shared by X" badge.
   */
  async function findShared(kp: KeyPair): Promise<AppFile[]> {
    if (!kp.input) {
      throw new Error('Cannot list shared items without private key')
    }

    const sharesApi = await import('../shares/api')
    const page = await sharesApi.getSharesMine()
    return Promise.all(page.items.map((row) => mapIncomingToFile(row, kp)))
  }

  /**
   * Decide where an updated row from `/api/storage/...` should land in the
   * recipient's view. The server is authoritative about ownership and the
   * real parent pointer, but the SPA injects a synthetic
   * `__shared_with_me__` parent for incoming shares — naive `updateItem`
   * with the server's `file_id` would route the row out of that virtual
   * folder. Preserve the synthetic placement when the existing row was
   * already there; let owned rows keep the server's parent.
   */
  function placeForRecipient(updated: AppFile, previous: AppFile): AppFile {
    if (previous.is_owner === false && previous.file_id === SHARED_WITH_ME_DIR_ID) {
      return { ...updated, file_id: SHARED_WITH_ME_DIR_ID }
    }
    return updated
  }

  async function mapIncomingToFile(row: IncomingShare, kp: KeyPair): Promise<AppFile> {
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
      md5: row.md5 ?? undefined,
      sha1: row.sha1 ?? undefined,
      sha256: row.sha256 ?? undefined,
      blake2b: row.blake2b ?? undefined,
      file_id: SHARED_WITH_ME_DIR_ID,
      file_modified_at: row.created_at,
      created_at: row.created_at,
      is_new: false,
      editable: row.editable,
      active_version: 1,
      encrypted_key: row.encrypted_key,
      encrypted_name: row.encrypted_name,
      encrypted_thumbnail: row.encrypted_thumbnail ?? undefined,
      cipher: row.cipher,
      share_role: row.share_role,
      shared_by_email: row.shared_by_email ?? row.owner_email,
      owner_email: row.owner_email,
      temporaryId: uuidv4()
    }

    try {
      const decrypted = await meta.decrypt(
        {
          cipher: row.cipher,
          encrypted_key: row.encrypted_key,
          encrypted_name: row.encrypted_name,
          encrypted_thumbnail: row.encrypted_thumbnail ?? undefined
        },
        kp.input as string
      )
      return { ...base, ...decrypted }
    } catch {
      // Leaving `name` at the file_id keeps the row navigable even when
      // a single share's key wrap is corrupt — matches the per-row
      // decrypt isolation called out in the plan.
      return base
    }
  }

  async function ensureSharedWithMeRoot(visible: boolean): Promise<void> {
    const existing = getItem(SHARED_WITH_ME_DIR_ID)
    if (!visible) {
      if (existing) removeItem(SHARED_WITH_ME_DIR_ID)
      return
    }
    if (existing) return

    addItem({
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
      cipher: '',
      temporaryId: uuidv4()
    })
  }

  /**
   * Attempts to avoid decrypting of an item that is already in the list.
   *
   * For recipient-side rows the server is authoritative about ownership but
   * blind to the SPA's `__shared_with_me__` virtual placement. When the
   * existing row lives under that synthetic parent, naive replacement
   * writes back the server's real `file_id` (often `null` for the share
   * root, or a folder the recipient has no row for inside a nested share)
   * and leaks the row out of the virtual folder into the recipient's owned
   * root. `placeForRecipient` rebinds the response to the virtual placement
   * so dedupe stays correct across owned/shared navigation.
   */
  async function replaceItem(item: AppFile, kp: KeyPair): Promise<void> {
    const existing = getItem(item.id)

    if (existing && existing.key) {
      return upsertItem(placeForRecipient({
        ...item,
        key: existing.key,
        name: existing.name,
        thumbnail: existing.thumbnail,
        temporaryId: uuidv4()
      }, existing))
    } else {
      const decrypted = await decryptItem({ ...item, temporaryId: uuidv4() }, kp)
      return upsertItem(existing ? placeForRecipient(decrypted, existing) : decrypted)
    }
  }

  /**
   * Decrypt each item
   */
  async function decryptItem(item: AppFile, kp: KeyPair): Promise<AppFile> {
    const decryptedParts = await meta.decrypt(item, kp.input as string)

    return {
      ...item,
      ...decryptedParts
    }
  }

  /**
   * Add or update a new item in the list
   */
  function upsertItem(item: AppFile): void {
    if (hasItem(item.id, item.file_id || null)) {
      updateItem(item)
    } else {
      addItem({ ...item, temporaryId: uuidv4() })
    }
  }

  /**
   * Get copy of the item from the list
   */
  function getItem(id: string): AppFile | null {
    const index = _items.value.findIndex((item) => item.id === id)

    return _items.value[index] || null
  }

  /**
   * Remove item from the list
   */
  function takeItem(id: string): AppFile | null {
    const index = _items.value.findIndex((item) => item.id === id)
    return _items.value.slice(index, 1)[0] || null
  }

  /**
   * Remove item from the list
   */
  function hasItem(id: string, file_id: string | null): boolean {
    return _items.value.findIndex((item) => item.id === id && item.file_id === file_id) !== -1
  }

  /**
   * Update existing item in the list
   */
  function updateItem(file: AppFile) {
    const index = _items.value.findIndex((item) => item.id === file.id)

    if (index === -1) {
      return
    }

    _items.value.splice(index, 1, file)
  }

  /**
   * Add new item to the list
   */
  function addItem(item: AppFile): void {
    _items.value.push(item)
  }

  /**
   * Remove item from the list
   */
  function removeItem(id: string): void {
    _items.value = _items.value.filter((item) => item.id !== id)
    selected.value = selected.value.filter((item) => item.id !== id)
  }

  /**
   * Adjust the cached `shared_with_count` on the row matching `id` so the
   * inline shared-out badge in the file browser refreshes without a full
   * listing reload. Floors at zero — the server is the source of truth and
   * a stale local view should never claim "shared with -1 accounts."
   */
  function bumpSharedWithCount(id: string, delta: number): void {
    const index = _items.value.findIndex((item) => item.id === id)
    if (index === -1) return
    const current = _items.value[index].shared_with_count ?? 0
    const next = Math.max(0, current + delta)
    _items.value[index] = { ..._items.value[index], shared_with_count: next }
  }

  /**
   * Download and decrypt file to the local machine
   */
  async function get(file: AppFile, kp: KeyPair): Promise<AppFile> {
    if (!file.id) {
      throw new Error('Cannot download file without ID')
    }

    if (file.mime === 'dir') {
      throw new Error('Cannot download directory')
    }

    if (!file.key) {
      throw new Error('Cannot download file without key')
    }

    return download.get(file, kp)
  }

  /**
   * Load file metadata, use the inner storage if the file is found, if not, fetch it from the backend.
   *
   * Recipient rows that originated from the `Shared with me` virtual root carry
   * `file_id = SHARED_WITH_ME_DIR_ID` in the cached store but `file_id = null`
   * on the server. Hand the response through `placeForRecipient` so callers
   * that consume `metadata` (preview navigation, action sheet) keep seeing the
   * virtual placement and route back into the virtual folder instead of the
   * recipient's owned root.
   */
  async function metadata(id: string, kp: KeyPair): Promise<AppFile> {
    const fetched = await meta.get(kp, id)
    const existing = getItem(id)
    return existing ? placeForRecipient(fetched, existing) : fetched
  }

  /**
   * Remove a single file from the storage
   */
  async function remove(kp: KeyPair, file: Partial<AppFile>): Promise<void> {
    if (!file.id) {
      throw new Error('Cannot remove file without ID')
    }

    await meta.remove(file.id)
    removeItem(file.id)

    await find(kp, fileId.value)
    emitFileTreeChange({ type: 'deleted', folderId: file.file_id || undefined })
  }

  /**
   * Delete many files from the list right away
   */
  async function removeAll(kp: KeyPair, files: AppFile[]): Promise<void> {
    await meta.removeAll({ ids: files.map((f) => f.id) })
    files.forEach((file) => removeItem(file.id))
    await find(kp, fileId.value, true)
    emitFileTreeChange({ type: 'deleted', folderId: fileId.value || undefined })
  }

  /**
   * Move many files into a new directory
   */
  async function moveAll(
    kp: KeyPair,
    files: AppFile[],
    file_id?: string | null | undefined
  ): Promise<void> {
    await meta.moveMany({ ids: files.map((f) => f.id), file_id })

    if (file_id !== fileId.value) {
      files.forEach((file) => removeItem(file.id))
    }

    await find(kp, fileId.value, true)
    emitFileTreeChange({ type: 'moved', folderId: fileId.value || undefined, targetFolderId: file_id || undefined })
  }

  /**
   * Create a directory in the storage
   */
  async function createDir(keypair: KeyPair, name: string, dir_id?: string): Promise<AppFile> {
    const search_tokens_hashed = cryptfns.stringToHashedTokens(name.toLowerCase())

    const createFile: CreateFile = {
      name,
      mime: 'dir',
      file_id: dir_id,
      file_modified_at: utcStringFromLocal(new Date()),
      search_tokens_hashed,
      cipher: cryptfns.cipher.DEFAULT_CIPHER
    }

    const dir = await meta.create(keypair, createFile)
    emitFileTreeChange({ type: 'created', folderId: dir_id })
    return dir
  }

  /**
   * Rename a file or directory. For rows the caller doesn't own (incoming
   * shares) the server returns the row keyed against its real parent, which
   * is either `null` or a folder the caller has no row for — surfacing it
   * verbatim would pop the renamed file out of `__shared_with_me__` and into
   * a directory the recipient can't navigate. `placeForRecipient` rebinds
   * the response to the same virtual placement the list endpoint computed
   * for the original row.
   */
  async function rename(keypair: KeyPair, file: AppFile, name: string): Promise<AppFile> {
    const search_tokens_hashed = cryptfns.stringToHashedTokens(name.toLowerCase())

    const renamed = await meta.rename(keypair, file, {
      name,
      search_tokens_hashed
    })

    const placed = placeForRecipient(renamed, file)
    updateItem(placed)
    emitFileTreeChange({ type: 'renamed', folderId: file.file_id || undefined })

    return placed
  }

  /**
   * Add single file to select list
   */
  function selectOne(select: boolean, file: AppFile) {
    if (select) {
      selected.value.push(file)
    } else {
      selected.value = selected.value.filter((f) => f.id !== file.id)
    }
  }

  /**
   * Add single file to select list
   */
  function selectAll(files: AppFile[], fileId?: string | null) {
    selected.value = files.filter((f) => {
      if (fileId && f.file_id !== fileId) {
        return false
      }

      return true
    })
  }

  /**
   * Add single file to select list
   */
  function deselectAll() {
    selected.value = []
  }

  /**
   * Drop every account-scoped value so a logout (or a switch to a different
   * account without a page reload) doesn't surface the previous user's
   * decrypted file list. Persisted per-directory sort prefs are device-level,
   * not account-level, so they're intentionally left intact.
   */
  function reset(): void {
    _items.value = []
    selected.value = []
    fileId.value = undefined
    error.value = null
    stats.value = undefined
    loading.value = false
  }

  /**
   * Set the sort value for a given directory
   */
  function setSort(dir: string, parameter: string, order: 'asc' | 'desc'): void {
    sort.value[dir] = `${parameter}|${order}`
  }

  /**
   * Simple version of sort that can be used in the UI
   */
  function setSortSimple(value: string): void {
    const [parameter, order] = value.split('|')

    setSort(fileId.value || 'root', parameter, order as 'asc' | 'desc')
  }

  /**
   * Get the sort value for given directory
   */
  function getSort(dir: string): string {
    return sort.value[dir] || 'name|desc'
  }

  return {
    addItem,
    bumpSharedWithCount,
    createDir,
    decryptItem,
    deselectAll,
    dir,
    directories,
    find,
    get,
    getItem,
    getSort,
    hasItem,
    items,
    loading,
    loadStats,
    metadata,
    moveAll,
    parameters,
    parents,
    placeForRecipient,
    remove,
    removeAll,
    removeItem,
    rename,
    replaceItem,
    reset,
    selectAll,
    selected,
    selectOne,
    sharedDirectories,
    sharedRoots,
    setSort,
    setSortSimple,
    sortOptions,
    stats,
    takeItem,
    title,
    updateItem,
    upsertItem
  }
})

/**
 * Do a full text search through the files and folders
 */
export async function search(
  query: string,
  kp: KeyPair,
  options?: { editable?: boolean; limit?: number }
): Promise<AppFile[]> {
  if (!kp.input) {
    throw new Error('Cannot search without private key')
  }

  const privateKey = kp.input
  const response = await meta.search(query, options)

  const results = await Promise.all(
    response.map(async (file: EncryptedAppFile) => {
      const unencryptedPart = await meta.decrypt(file, privateKey)

      return {
        ...file,
        ...unencryptedPart
      }
    })
  )

  return results
}
