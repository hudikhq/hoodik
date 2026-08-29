import * as meta from './meta'
import * as queue from '../queue'
import * as thumbnailCache from './thumbnail-cache'
import * as upload from './upload'
import * as download from './download'
import { downloadAndDecrypt } from './download/sync'
import { rankSearchResults } from './rank'
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
import { SHARED_WITH_ME_DIR_ID } from '../constants'
export { SHARED_WITH_ME_DIR_ID }

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
const collator = new Intl.Collator()

function innerSort(items: AppFile[], parameter: string): AppFile[] {
  return items.sort((a, b) => {
    // @ts-ignore
    const aValue = a[parameter] || ''
    // @ts-ignore
    const bValue = b[parameter] || ''

    if (typeof aValue === 'number' && typeof bValue === 'number') {
      return aValue - bValue
    }

    return collator.compare(aValue, bValue)
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

  // Resolved once the first root listing has fully landed in `_items` —
  // rows decrypted and upserted, shares probe done. The sidebar tree seeds
  // its first render from this instead of racing the main view with a
  // duplicate root request and a second decrypt pass.
  let resolveFirstRootListed: (() => void) | null = null
  const firstRootListed = new Promise<void>((resolve) => (resolveFirstRootListed = resolve))

  async function firstRootListing(): Promise<AppFile[]> {
    await firstRootListed
    return _items.value.filter((item) => !item.file_id)
  }

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

  let statsFetchedAt = 0

  /**
   * Load the storage stats. Every folder click used to refetch these —
   * two server-side aggregates per navigation for a quota bar that only
   * moves on writes. The TTL keeps navigation free; writes pass `force`.
   */
  async function loadStats(force = false): Promise<void> {
    if (!force && stats.value && Date.now() - statsFetchedAt < 15_000) return

    stats.value = await meta.stats()
    statsFetchedAt = Date.now()
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
        error.value = 'errors.listFailed'
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
      error.value = 'errors.listFailed'
    }

    const rows = [...(response.parents || []), ...(response.children || [])]
    const upserts = Promise.all(rows.map((item) => prepareItem(item, kp))).then((prepared) =>
      upsertMany(prepared)
    )

    // The rows are already streaming in — the shares probe below only
    // decides whether the synthetic "Shared with me" folder appears, so
    // it must not hold the listing spinner for its own round trip.
    loading.value = false

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

      if (resolveFirstRootListed) {
        const resolve = resolveFirstRootListed
        resolveFirstRootListed = null
        upserts.then(resolve, resolve)
      }
    }
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
      has_thumbnail: row.has_thumbnail ?? false,
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
          encrypted_name: row.encrypted_name
        },
        kp.wrappingPrivate || (kp.input as string)
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
    upsertItem(await prepareItem(item, kp))
  }

  /**
   * Decrypt/merge a server row into its store-ready shape without writing
   * it — listings prepare every row first and land them in one mutation,
   * so the sorted view recomputes once instead of once per row.
   */
  async function prepareItem(item: AppFile, kp: KeyPair): Promise<AppFile> {
    const existing = getItem(item.id)

    if (existing && existing.key) {
      return placeForRecipient({
        ...item,
        key: existing.key,
        name: existing.name,
        thumbnail: existing.thumbnail,
        temporaryId: uuidv4()
      }, existing)
    }

    const decrypted = await decryptItem({ ...item, temporaryId: uuidv4() }, kp)
    return existing ? placeForRecipient(decrypted, existing) : decrypted
  }

  /**
   * Decrypt each item
   */
  async function decryptItem(item: AppFile, kp: KeyPair): Promise<AppFile> {
    const decryptedParts = await meta.decrypt(item, kp.wrappingPrivate || (kp.input as string))

    return {
      ...item,
      ...decryptedParts
    }
  }

  /**
   * In-flight thumbnail fetches keyed by file id. The same file can mount
   * in the file browser and the sidebar tree in one tick — the map
   * collapses their fetches into a single request.
   */
  const thumbnailFetches = new Map<string, Promise<string | undefined>>()

  /**
   * Fetch and decrypt a file's thumbnail on demand. Listings request an
   * `attributes` projection without the blob and only carry
   * `has_thumbnail`; the ciphertext comes from the localStorage cache
   * when a previous session already fetched it, or from the thumbnail
   * route otherwise. The decrypted result is cached back onto the store
   * row so navigation, previews and link creation reuse it without
   * another decrypt.
   */
  async function loadThumbnail(file: AppFile): Promise<string | undefined> {
    if (file.thumbnail) return file.thumbnail

    const cached = getItem(file.id)
    if (cached?.thumbnail) return cached.thumbnail

    if (!file.has_thumbnail || !file.key) return undefined

    const pending = thumbnailFetches.get(file.id)
    if (pending) return pending

    const key = file.key
    const fetched = (async () => {
      const cachedCiphertext = thumbnailCache.get(file.id, file.active_version)
      const encrypted = cachedCiphertext ?? (await meta.thumbnail(file.id))
      if (!encrypted) return undefined

      const thumbnail = await cryptfns.cipher.decryptString(file.cipher, encrypted, key)
      if (!cachedCiphertext) {
        thumbnailCache.put(file.id, file.active_version, encrypted)
      }

      const existing = getItem(file.id)
      if (existing) updateItem({ ...existing, thumbnail })

      return thumbnail
    })()

    thumbnailFetches.set(file.id, fetched)
    try {
      return await fetched
    } finally {
      thumbnailFetches.delete(file.id)
    }
  }

  /**
   * Add or update a new item in the list. Matched on id alone, like
   * `upsertMany` — a row whose placement changed (a shared folder first
   * seen under its real parent, then re-listed under the synthetic
   * `__shared_with_me__` folder) replaces the existing copy instead of
   * landing next to it and rendering the same file twice.
   */
  function upsertItem(item: AppFile): void {
    if (getItem(item.id)) {
      updateItem(item)
    } else {
      addItem({ ...item, temporaryId: uuidv4() })
    }
  }

  /**
   * Land a whole listing in one reactive mutation — every dependent
   * computed (the sorted view above all) recomputes once, not per row.
   */
  function upsertMany(files: AppFile[]): void {
    const next = [..._items.value]

    for (const file of files) {
      const index = next.findIndex((item) => item.id === file.id)

      if (index === -1) {
        next.push({ ...file, temporaryId: uuidv4() })
      } else {
        next.splice(index, 1, file)
      }
    }

    _items.value = next
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

    await find(kp, fileId.value, false)
    emitFileTreeChange({ type: 'deleted', folderId: file.file_id || undefined })
  }

  /**
   * Delete many files from the list right away
   */
  async function removeAll(kp: KeyPair, files: AppFile[]): Promise<void> {
    await meta.removeAll({ ids: files.map((f) => f.id) })
    files.forEach((file) => removeItem(file.id))
    await find(kp, fileId.value, false)
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

    await find(kp, fileId.value, false)
    emitFileTreeChange({ type: 'moved', folderId: fileId.value || undefined, targetFolderId: file_id || undefined })
  }

  /**
   * Nearest ancestor-or-self folder of `dirId` that carries a signed
   * member list — the folder whose list authorises a multi-key write
   * anywhere in its subtree. Folders below a share root hold the root's
   * roster (fan-out, cascade moves and multi-key creates all copy it)
   * but no signature of their own, and the server rejects any write
   * whose wrap set doesn't match the actual target's rows, so resolving
   * at the root is exactly as strong as writing into the root itself.
   *
   * Walks the store first — the breadcrumb chain of any folder the user
   * navigated into is already there — and falls back to one listing
   * fetch for rows the store doesn't hold (a move-picker destination).
   * Returns null for a private tree or a pre-signature legacy share.
   */
  async function resolveRosterFolder(
    dirId: string | null | undefined
  ): Promise<AppFile | null> {
    if (!dirId || dirId === SHARED_WITH_ME_DIR_ID) return null

    const seen = new Set<string>()
    let cursor: string | null = dirId
    while (cursor && cursor !== SHARED_WITH_ME_DIR_ID && !seen.has(cursor)) {
      seen.add(cursor)
      const row = getItem(cursor)
      if (!row) {
        try {
          const response = await meta.find({ dir_id: dirId })
          const parents = (response.parents ?? []) as AppFile[]
          // `parents` runs root-first and includes the target itself, so
          // the nearest signed folder is the last match.
          for (let i = parents.length - 1; i >= 0; i--) {
            if (parents[i].members_signed_at != null) return parents[i]
          }
        } catch {
          // Unknown folder — treated the same as an unsigned chain.
        }
        return null
      }
      if (row.mime !== 'dir') return null
      if (row.members_signed_at != null) return row
      cursor = row.file_id ?? null
    }
    return null
  }

  /**
   * Roster source for a write into `dirId`, or undefined for a plain
   * owner-only write. A non-owned folder with no signed list anywhere in
   * its chain (a pre-signature legacy share) falls back to the folder
   * itself, so those writes keep today's verification error instead of
   * silently producing an owner-only row.
   */
  async function writeRosterId(
    dirId: string | null | undefined
  ): Promise<string | undefined> {
    if (!dirId || dirId === SHARED_WITH_ME_DIR_ID) return undefined
    const resolved = await resolveRosterFolder(dirId)
    if (resolved) return resolved.id
    const row = getItem(dirId)
    return row && row.mime === 'dir' && row.is_owner === false ? row.id : undefined
  }

  /**
   * Create a directory in the storage. A parent inside a shared tree
   * routes through the multi-key create so every member receives a wrap
   * of the folder key — the regular create would leave the new directory
   * visible to the creator alone. `rosterFolderId` carries a caller-
   * resolved roster source (folder uploads resolve once for the whole
   * batch); without it the parent's chain is resolved here.
   */
  async function createDir(
    keypair: KeyPair,
    name: string,
    dir_id?: string,
    callerUserId?: string,
    rosterFolderId?: string
  ): Promise<AppFile> {
    const parent = dir_id ? getItem(dir_id) : undefined
    const roster = rosterFolderId ?? (parent ? await writeRosterId(dir_id) : undefined)

    if (parent && roster != null) {
      if (!callerUserId) {
        throw new Error('Cannot create directory in a shared folder without caller id')
      }
      // Loaded lazily for the same reason `find` pulls the shares API in
      // on demand — the multi-key pipeline stays out of the boot bundle.
      const save = await import('./save')
      const dir = await save.createDirInSharedFolder(
        keypair,
        name,
        parent as AppFile,
        callerUserId,
        roster
      )
      upsertItem(dir)
      emitFileTreeChange({ type: 'created', folderId: dir_id })
      return dir
    }

    const createFile: CreateFile = {
      name,
      mime: 'dir',
      file_id: dir_id,
      file_modified_at: utcStringFromLocal(new Date()),
      cipher: cryptfns.cipher.defaultCipher()
    }

    const dir = await meta.create(keypair, createFile)
    upsertItem(dir)
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
    const renamed = await meta.rename(keypair, file, { name })

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
    error,
    find,
    firstRootListing,
    getItem,
    getSort,
    items,
    loading,
    loadStats,
    loadThumbnail,
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
    resolveRosterFolder,
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
    upsertItem,
    writeRosterId
  }
})

/**
 * File keys for every file shared with this account, cached for the session.
 *
 * Search tags a shared file under the file's own key, so a query has to carry
 * one tag set per such file. `/api/shares/keys` returns the untrimmed set —
 * `/api/shares/mine` reports roots only, which would leave everything inside a
 * shared folder unsearchable.
 *
 * Cached because it costs one asymmetric unwrap per shared file, but only for
 * a few minutes: the set changes when a share is granted or revoked, and the
 * grant that matters most is someone else's — which this client only learns
 * about by asking again. A short expiry makes a freshly shared file
 * searchable within minutes instead of after the next login.
 * `clearIncomingSearchKeys` drops the cache on logout, so the keys never
 * outlive the session that unlocked them.
 */
let incomingKeyCache: { keys: Uint8Array[]; fetchedAt: number } | null = null

const INCOMING_KEYS_TTL_MS = 5 * 60 * 1000

export function clearIncomingSearchKeys() {
  incomingKeyCache = null
}

async function incomingSearchKeys(kp: KeyPair): Promise<Uint8Array[]> {
  if (incomingKeyCache && Date.now() - incomingKeyCache.fetchedAt < INCOMING_KEYS_TTL_MS) {
    return incomingKeyCache.keys
  }

  const privateKey = kp.wrappingPrivate || kp.input
  if (!privateKey) return []

  try {
    const sharesApi = await import('../shares/api')
    const rows = await sharesApi.getIncomingKeys()

    const keys = await Promise.all(
      rows.map(async (row) => {
        try {
          return await meta.decryptFileKey(row.encrypted_key, privateKey)
        } catch {
          // A row wrapped under a superseded key is not worth failing the
          // whole search over; it simply will not match.
          return undefined
        }
      })
    )

    incomingKeyCache = {
      keys: keys.filter((key): key is Uint8Array => !!key),
      fetchedAt: Date.now()
    }

    return incomingKeyCache.keys
  } catch {
    // Search over owned files is the common case and must not fail because
    // the shares list is unavailable.
    return []
  }
}

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

  const privateKey = kp.wrappingPrivate || kp.input

  // Files shared with the caller are tagged under each file's own key, so the
  // query has to carry one tag set per such file. Only the shares the caller
  // holds directly are reachable here — see the note on `incomingSearchKeys`.
  const sharedKeys = await incomingSearchKeys(kp)
  const response = await meta.search(query, kp, sharedKeys, options)

  const results = await Promise.all(
    response.map(async (file: EncryptedAppFile) => {
      const unencryptedPart = await meta.decrypt(file, privateKey)

      return {
        ...file,
        ...unencryptedPart
      }
    })
  )

  // Server recall, client precision: hydrate candidate note bodies and
  // re-rank against the plaintext only this side can see.
  const bodies = await hydrateNoteBodies(results)

  return rankSearchResults(query, results, bodies)
}

/** Notes above this size are ranked on their name and server evidence only. */
const HYDRATE_MAX_BYTES = 512 * 1024
/** How many note candidates get their body loaded per search. */
const HYDRATE_MAX_NOTES = 20
/** Parallel body downloads per batch. */
const HYDRATE_CONCURRENCY = 4

/**
 * Download and decrypt the bodies of the note rows among [results] so the
 * refinement pass can score real content matches. Best-effort: a body that
 * fails to load leaves its row scored on name and server evidence alone.
 */
async function hydrateNoteBodies(results: AppFile[]): Promise<Map<string, string>> {
  const bodies = new Map<string, string>()
  const candidates = results
    .filter((file) => file.editable && file.mime !== 'dir' && (file.size || 0) <= HYDRATE_MAX_BYTES)
    .slice(0, HYDRATE_MAX_NOTES)

  for (let i = 0; i < candidates.length; i += HYDRATE_CONCURRENCY) {
    await Promise.all(
      candidates.slice(i, i + HYDRATE_CONCURRENCY).map(async (file) => {
        try {
          const bytes = await downloadAndDecrypt(file)
          bodies.set(file.id, new TextDecoder().decode(bytes))
        } catch (
          // eslint-disable-next-line @typescript-eslint/no-unused-vars
          _
        ) {
          // Name-only scoring for this row.
        }
      })
    )
  }

  return bodies
}
