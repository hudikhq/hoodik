import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import * as meta from '../../services/storage/meta'
import * as sharesApi from '../../services/shares/api'
import { store as storageStore, SHARED_WITH_ME_DIR_ID } from '../../services/storage'

import type { IncomingSharePage, KeyPair } from '../../types'

const KEYPAIR: KeyPair = { input: 'private-key' } as KeyPair

function makeIncomingPage(overrides: Partial<IncomingSharePage> = {}): IncomingSharePage {
  return {
    items: [
      {
        file_id: '11111111-1111-1111-1111-111111111111',
        mime: 'text/plain',
        encrypted_name: 'cipher-blob',
        cipher: 'aegis128l',
        editable: false,
        share_role: 'reader',
        encrypted_key: 'wrapped-key',
        created_at: 1_700_000_000,
        shared_at: 1_700_000_500,
        owner_id: 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
        owner_email: 'alice@example.com',
        owner_pubkey: 'pub',
        owner_pubkey_fingerprint: 'fp',
        shared_by_user_id: 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
        shared_by_email: 'alice@example.com'
      }
    ],
    total: 1,
    limit: 50,
    offset: 0,
    ...overrides
  }
}

const EMPTY_PAGE: IncomingSharePage = { items: [], total: 0, limit: 50, offset: 0 }

describe('Storage store: bumpSharedWithCount (B2)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('bumpSharedWithCount increments a known row', () => {
    const store = storageStore()
    store.addItem({
      id: 'aaa',
      file_id: null,
      mime: 'image/png',
      name: 'pic.png',
      is_owner: true,
      shared_with_count: 0
    } as never)
    store.bumpSharedWithCount('aaa', 2)
    expect(store.getItem('aaa')?.shared_with_count).toEqual(2)
  })

  it('bumpSharedWithCount decrements without going negative', () => {
    const store = storageStore()
    store.addItem({
      id: 'aaa',
      file_id: null,
      mime: 'image/png',
      name: 'pic.png',
      is_owner: true,
      shared_with_count: 1
    } as never)
    store.bumpSharedWithCount('aaa', -5)
    expect(store.getItem('aaa')?.shared_with_count).toEqual(0)
  })

  it('bumpSharedWithCount on an unknown id is a no-op', () => {
    const store = storageStore()
    expect(() => store.bumpSharedWithCount('missing', 1)).not.toThrow()
  })
})

beforeEach(() => {
  setActivePinia(createPinia())
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('Storage store: Shared with me virtual folder', () => {
  it('storage_findShared_maps_incoming_to_rows_under_synthetic_dir', async () => {
    vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(makeIncomingPage())
    vi.spyOn(meta, 'decrypt').mockResolvedValue({ name: 'plaintext.txt' })

    const store = storageStore()
    await store.find(KEYPAIR, SHARED_WITH_ME_DIR_ID)

    const rows = store.items.filter((row) => row.id !== SHARED_WITH_ME_DIR_ID)
    expect(rows).toHaveLength(1)
    expect(rows[0].file_id).toEqual(SHARED_WITH_ME_DIR_ID)
    expect(rows[0].name).toEqual('plaintext.txt')
    expect(rows[0].shared_by_email).toEqual('alice@example.com')
    expect(rows[0].owner_email).toEqual('alice@example.com')
    expect(rows[0].is_owner).toBe(false)
    expect(rows[0].share_role).toEqual('reader')
  })

  it('storage_root_listing_injects_synthetic_folder_when_incoming_exists', async () => {
    vi.spyOn(meta, 'find').mockResolvedValue({ children: [], parents: [] })
    vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(makeIncomingPage())

    const store = storageStore()
    await store.find(KEYPAIR, undefined)

    const synthetic = store.items.find((row) => row.id === SHARED_WITH_ME_DIR_ID)
    expect(synthetic).toBeTruthy()
    expect(synthetic?.mime).toEqual('dir')
    expect(synthetic?.name).toEqual('Shared with me')
    expect(synthetic?.file_id).toEqual(null)
  })

  it('storage_root_listing_omits_synthetic_folder_when_no_incoming', async () => {
    vi.spyOn(meta, 'find').mockResolvedValue({ children: [], parents: [] })
    vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(EMPTY_PAGE)

    const store = storageStore()
    await store.find(KEYPAIR, undefined)

    expect(store.items.find((row) => row.id === SHARED_WITH_ME_DIR_ID)).toBeUndefined()
  })

  it('storage_findShared_decrypt_failure_keeps_row_navigable', async () => {
    // A corrupt key wrap on one share row must not blank the whole list;
    // the failed row stays renderable with the file_id as a name fallback.
    vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(makeIncomingPage())
    vi.spyOn(meta, 'decrypt').mockRejectedValue(new Error('boom'))

    const store = storageStore()
    await store.find(KEYPAIR, SHARED_WITH_ME_DIR_ID)

    const rows = store.items.filter((row) => row.id !== SHARED_WITH_ME_DIR_ID)
    expect(rows).toHaveLength(1)
    expect(rows[0].name).toEqual(rows[0].id)
    expect(rows[0].shared_by_email).toEqual('alice@example.com')
  })

  it('storage_findShared_carries_size_and_upload_progress', async () => {
    // DetailsModal and TableFileRow read these fields straight off the
    // AppFile; if the synthetic mapping drops them the recipient sees
    // "Size: 0 B" + a never-finishing upload bar on content the owner
    // long since uploaded.
    vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(
      makeIncomingPage({
        items: [
          {
            file_id: '11111111-1111-1111-1111-111111111111',
            mime: 'text/plain',
            encrypted_name: 'cipher-blob',
            cipher: 'aegis128l',
            editable: false,
            size: 4096,
            chunks: 4,
            chunks_stored: 4,
            finished_upload_at: 1_700_001_000,
            md5: 'md5-hash',
            sha1: 'sha1-hash',
            sha256: 'sha256-hash',
            blake2b: 'blake2b-hash',
            share_role: 'reader',
            encrypted_key: 'wrapped-key',
            created_at: 1_700_000_000,
            shared_at: 1_700_000_500,
            owner_id: 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
            owner_email: 'alice@example.com',
            owner_pubkey: 'pub',
            owner_pubkey_fingerprint: 'fp',
            shared_by_user_id: 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
            shared_by_email: 'alice@example.com'
          }
        ]
      })
    )
    vi.spyOn(meta, 'decrypt').mockResolvedValue({ name: 'doc.txt' })

    const store = storageStore()
    await store.find(KEYPAIR, SHARED_WITH_ME_DIR_ID)

    const row = store.items.find((r) => r.id === '11111111-1111-1111-1111-111111111111')
    expect(row).toBeTruthy()
    expect(row?.size).toEqual(4096)
    expect(row?.chunks).toEqual(4)
    expect(row?.chunks_stored).toEqual(4)
    expect(row?.finished_upload_at).toEqual(1_700_001_000)
    expect(row?.md5).toEqual('md5-hash')
    expect(row?.sha1).toEqual('sha1-hash')
    expect(row?.sha256).toEqual('sha256-hash')
    expect(row?.blake2b).toEqual('blake2b-hash')
  })

  it('storage_root_pins_synthetic_folder_first_ignoring_sort', async () => {
    // The synthetic folder is an injected affordance, not user content.
    // Sorting it alphabetically buries it between owned folders whose
    // names happen to alphabetize earlier (e.g. "academica.lux"); the
    // user can't tell at a glance that the folder is theirs to open.
    // Pin it to index 0 of the root listing instead, regardless of which
    // direction the user is currently sorting by.
    vi.spyOn(meta, 'find').mockResolvedValue({
      children: [
        {
          id: 'dir-academica',
          file_id: null,
          mime: 'dir',
          name: 'academica.lux',
          encrypted_name: '',
          encrypted_key: '',
          cipher: 'aegis128l'
        },
        {
          id: 'dir-zebra',
          file_id: null,
          mime: 'dir',
          name: 'zebra',
          encrypted_name: '',
          encrypted_key: '',
          cipher: 'aegis128l'
        }
      ],
      parents: []
    } as unknown as Awaited<ReturnType<typeof meta.find>>)
    vi.spyOn(meta, 'decrypt').mockImplementation(async (item) => {
      const id = (item as { id?: string }).id
      if (id === 'dir-academica') return { name: 'academica.lux' }
      if (id === 'dir-zebra') return { name: 'zebra' }
      return { name: '' }
    })
    vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(makeIncomingPage())

    const store = storageStore()
    await store.find(KEYPAIR, undefined)

    // The list ordering check has to wait for the after-mock async
    // upserts to land in the reactive store. A single tick is enough
    // for the synthetic injection that fires after `meta.find` returns.
    await Promise.resolve()
    await Promise.resolve()

    // Default sort is name|desc — under that order owned folders go
    // zebra, academica.lux. The synthetic still has to be first.
    let topThree = store.items.slice(0, 3).map((row) => row.id)
    expect(topThree[0]).toEqual(SHARED_WITH_ME_DIR_ID)
    expect(topThree.slice(1)).toEqual(['dir-zebra', 'dir-academica'])

    // Flip the sort to ascending; the synthetic still has to be first.
    store.setSort('root', 'name', 'asc')
    topThree = store.items.slice(0, 3).map((row) => row.id)
    expect(topThree[0]).toEqual(SHARED_WITH_ME_DIR_ID)
    expect(topThree.slice(1)).toEqual(['dir-academica', 'dir-zebra'])
  })

  it('storage_rename_of_recipient_row_preserves_synthetic_parent', async () => {
    // The server returns the row keyed on its real parent (often `null`
    // for top-level shared files), which would pop the renamed entry out
    // of `__shared_with_me__` and into the recipient's actual root.
    // `placeForRecipient` keeps the synthetic placement so the row stays
    // where the user expects.
    vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(makeIncomingPage())
    vi.spyOn(meta, 'decrypt').mockResolvedValue({ name: 'old.txt' })
    vi.spyOn(meta, 'rename').mockResolvedValue({
      id: '11111111-1111-1111-1111-111111111111',
      file_id: null,
      is_owner: false,
      mime: 'text/plain',
      name: 'renamed.txt',
      encrypted_name: 'cipher',
      encrypted_key: 'wrapped',
      cipher: 'aegis128l'
    } as unknown as Awaited<ReturnType<typeof meta.rename>>)

    const store = storageStore()
    await store.find({ ...KEYPAIR, publicKey: 'pub' } as KeyPair, SHARED_WITH_ME_DIR_ID)
    const original = store.items.find(
      (r) => r.id === '11111111-1111-1111-1111-111111111111'
    )
    expect(original?.file_id).toEqual(SHARED_WITH_ME_DIR_ID)

    const renamed = await store.rename(
      { ...KEYPAIR, publicKey: 'pub' } as KeyPair,
      original!,
      'renamed.txt'
    )

    expect(renamed.file_id).toEqual(SHARED_WITH_ME_DIR_ID)
    const stored = store.items.find(
      (r) => r.id === '11111111-1111-1111-1111-111111111111'
    )
    expect(stored?.file_id).toEqual(SHARED_WITH_ME_DIR_ID)
    expect(stored?.name).toEqual('renamed.txt')
  })

  it('storage_rename_of_owned_row_keeps_server_parent', async () => {
    // The placement override is recipient-only; renaming an owned row
    // must still route through the server's parent pointer so moves /
    // breadcrumbs continue to work.
    vi.spyOn(meta, 'find').mockResolvedValue({ children: [], parents: [] })
    vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(EMPTY_PAGE)

    const store = storageStore()
    await store.find({ ...KEYPAIR, publicKey: 'pub' } as KeyPair, undefined)
    store.addItem({
      id: '33333333-3333-3333-3333-333333333333',
      user_id: '',
      is_owner: true,
      name: 'mine.txt',
      name_hash: '',
      mime: 'text/plain',
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
    } as never)

    vi.spyOn(meta, 'rename').mockResolvedValue({
      id: '33333333-3333-3333-3333-333333333333',
      file_id: 'real-parent',
      is_owner: true,
      mime: 'text/plain',
      name: 'renamed.txt',
      encrypted_name: 'cipher',
      encrypted_key: 'wrapped',
      cipher: 'aegis128l'
    } as unknown as Awaited<ReturnType<typeof meta.rename>>)

    const original = store.getItem('33333333-3333-3333-3333-333333333333')
    const renamed = await store.rename(
      { ...KEYPAIR, publicKey: 'pub' } as KeyPair,
      original!,
      'renamed.txt'
    )
    expect(renamed.file_id).toEqual('real-parent')
  })

  it('storage_placeForRecipient_helper_only_rewrites_synthetic_rows', () => {
    const store = storageStore()
    const recipientRow = {
      id: 'aaa',
      file_id: SHARED_WITH_ME_DIR_ID,
      is_owner: false
    } as unknown as Parameters<typeof store.placeForRecipient>[0]
    const fromServer = {
      id: 'aaa',
      file_id: null,
      is_owner: false
    } as unknown as Parameters<typeof store.placeForRecipient>[0]
    const placed = store.placeForRecipient(fromServer, recipientRow)
    expect(placed.file_id).toEqual(SHARED_WITH_ME_DIR_ID)

    // Recipient row that lives inside a real shared folder (not the
    // synthetic root) must keep the server's parent — that's a real
    // folder id the recipient has a row for.
    const nestedPrevious = {
      id: 'bbb',
      file_id: 'real-shared-folder',
      is_owner: false
    } as unknown as Parameters<typeof store.placeForRecipient>[0]
    const nestedFromServer = {
      id: 'bbb',
      file_id: 'real-shared-folder',
      is_owner: false
    } as unknown as Parameters<typeof store.placeForRecipient>[0]
    expect(store.placeForRecipient(nestedFromServer, nestedPrevious).file_id).toEqual(
      'real-shared-folder'
    )

    const owned = {
      id: 'ccc',
      file_id: 'real-parent',
      is_owner: true
    } as unknown as Parameters<typeof store.placeForRecipient>[0]
    expect(store.placeForRecipient(owned, owned).file_id).toEqual('real-parent')
  })

  it('storage_navigating_into_shared_folder_does_not_duplicate_row', async () => {
    // Bob enters `__shared_with_me__`, drills into a shared folder X, then
    // navigates back. Before the fix, `replaceItem` wrote the server's
    // `file_id` (null for a top-level share) into the cached row,
    // creating a second copy with the wrong placement. The synthetic
    // listing then carried two entries for the same id, and the duplicate
    // with `file_id=null` later leaked into Bob's owned root.
    vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(makeIncomingPage())
    vi.spyOn(meta, 'decrypt').mockImplementation(async (item) => {
      const id = (item as { id?: string }).id
      if (id === '11111111-1111-1111-1111-111111111111') return { name: 'shared-x' }
      return { name: '' }
    })
    vi.spyOn(meta, 'find').mockResolvedValue({
      children: [],
      parents: [
        {
          id: '11111111-1111-1111-1111-111111111111',
          file_id: null,
          is_owner: false,
          mime: 'dir',
          name: '',
          encrypted_name: '',
          encrypted_key: '',
          cipher: 'aegis128l'
        }
      ]
    } as unknown as Awaited<ReturnType<typeof meta.find>>)

    const store = storageStore()
    await store.find(KEYPAIR, SHARED_WITH_ME_DIR_ID)
    // Drill in — server-side, X's `file_id` is null.
    await store.find(KEYPAIR, '11111111-1111-1111-1111-111111111111')
    // Walk back to the virtual folder.
    vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(makeIncomingPage())
    await store.find(KEYPAIR, SHARED_WITH_ME_DIR_ID)

    const rows = store.items.filter(
      (r) => r.id === '11111111-1111-1111-1111-111111111111'
    )
    expect(rows).toHaveLength(1)
    expect(rows[0].file_id).toEqual(SHARED_WITH_ME_DIR_ID)
  })

  it('storage_owned_root_excludes_previously_visited_shared_folder', async () => {
    // After clicking through `__shared_with_me__` into a shared folder X
    // and then back to the owned root via the sidebar Files entry, X
    // must not appear in the recipient's own root listing. The server
    // never returns it there (the default root listing filters
    // `is_owner=true`), but a duplicate `file_id=null` row left behind
    // by the previous navigation used to surface it.
    vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(makeIncomingPage())
    vi.spyOn(meta, 'decrypt').mockResolvedValue({ name: 'shared-x' })
    vi.spyOn(meta, 'find')
      .mockResolvedValueOnce({
        children: [],
        parents: [
          {
            id: '11111111-1111-1111-1111-111111111111',
            file_id: null,
            is_owner: false,
            mime: 'dir',
            name: '',
            encrypted_name: '',
            encrypted_key: '',
            cipher: 'aegis128l'
          }
        ]
      } as unknown as Awaited<ReturnType<typeof meta.find>>)
      .mockResolvedValueOnce({ children: [], parents: [] })

    const store = storageStore()
    await store.find(KEYPAIR, SHARED_WITH_ME_DIR_ID)
    await store.find(KEYPAIR, '11111111-1111-1111-1111-111111111111')
    vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(makeIncomingPage())
    await store.find(KEYPAIR, undefined)

    const rootRows = store.items.filter((r) => r.id !== SHARED_WITH_ME_DIR_ID)
    expect(rootRows).toHaveLength(0)
  })

  it('storage_navigation_to_new_parent_clears_selection', async () => {
    // Selecting a row in `__shared_with_me__` and then walking to a new
    // parent (sidebar Files → own root) must reset the selection. Holding
    // it across the navigation surfaces a phantom checked checkbox on
    // whatever row happens to share the previously-selected file id.
    vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(makeIncomingPage())
    vi.spyOn(meta, 'decrypt').mockResolvedValue({ name: 'shared-x' })
    vi.spyOn(meta, 'find').mockResolvedValue({ children: [], parents: [] })

    const store = storageStore()
    await store.find(KEYPAIR, SHARED_WITH_ME_DIR_ID)
    const row = store.items.find((r) => r.id !== SHARED_WITH_ME_DIR_ID)
    expect(row).toBeTruthy()
    store.selectOne(true, row!)
    expect(store.selected.length).toBe(1)

    await store.find(KEYPAIR, undefined)
    expect(store.selected.length).toBe(0)
  })

  it('storage_metadata_preserves_synthetic_parent_for_recipient_rows', async () => {
    // The preview view feeds `metadata()` into FilePreview to compute
    // sibling-counter and the exit route. Without `placeForRecipient` the
    // recipient's top-level shared file came back with `file_id = null`,
    // so the counter sized the wrong folder (0/0) and the exit nav routed
    // to the recipient's owned root instead of `__shared_with_me__`.
    vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(makeIncomingPage())
    vi.spyOn(meta, 'decrypt').mockResolvedValue({ name: 'shared.png' })
    vi.spyOn(meta, 'get').mockResolvedValue({
      id: '11111111-1111-1111-1111-111111111111',
      file_id: null,
      is_owner: false,
      mime: 'image/png',
      name: 'shared.png',
      encrypted_name: 'cipher',
      encrypted_key: 'wrapped',
      cipher: 'aegis128l'
    } as unknown as Awaited<ReturnType<typeof meta.get>>)

    const store = storageStore()
    await store.find(KEYPAIR, SHARED_WITH_ME_DIR_ID)

    const result = await store.metadata('11111111-1111-1111-1111-111111111111', KEYPAIR)
    expect(result.file_id).toEqual(SHARED_WITH_ME_DIR_ID)
  })

  it('storage_metadata_keeps_server_parent_for_owned_rows', async () => {
    // Owned rows never went through `placeForRecipient`, so the metadata
    // response has to thread through unchanged.
    vi.spyOn(meta, 'find').mockResolvedValue({ children: [], parents: [] })
    vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(EMPTY_PAGE)
    vi.spyOn(meta, 'get').mockResolvedValue({
      id: '44444444-4444-4444-4444-444444444444',
      file_id: 'real-parent',
      is_owner: true,
      mime: 'image/png',
      name: 'mine.png',
      encrypted_name: 'cipher',
      encrypted_key: 'wrapped',
      cipher: 'aegis128l'
    } as unknown as Awaited<ReturnType<typeof meta.get>>)

    const store = storageStore()
    await store.find(KEYPAIR, undefined)
    store.addItem({
      id: '44444444-4444-4444-4444-444444444444',
      user_id: '',
      is_owner: true,
      name: 'mine.png',
      name_hash: '',
      mime: 'image/png',
      chunks: 0,
      file_id: 'real-parent',
      file_modified_at: 0,
      created_at: 0,
      is_new: false,
      editable: false,
      active_version: 1,
      encrypted_key: '',
      encrypted_name: '',
      cipher: ''
    } as never)

    const result = await store.metadata('44444444-4444-4444-4444-444444444444', KEYPAIR)
    expect(result.file_id).toEqual('real-parent')
  })

  it('storage_findShared_handles_in_progress_upload_metadata', async () => {
    // Mid-upload owner state — chunks_stored < chunks, no
    // finished_upload_at. The synthetic row has to surface both so the
    // recipient's progress bar renders the right percentage instead of
    // either 0% or 100%.
    vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(
      makeIncomingPage({
        items: [
          {
            file_id: '22222222-2222-2222-2222-222222222222',
            mime: 'application/octet-stream',
            encrypted_name: 'cipher',
            cipher: 'aegis128l',
            editable: false,
            size: 10_000,
            chunks: 10,
            chunks_stored: 3,
            finished_upload_at: null,
            md5: null,
            sha1: null,
            sha256: null,
            blake2b: null,
            share_role: 'reader',
            encrypted_key: 'wrapped',
            created_at: 1_700_000_000,
            shared_at: 1_700_000_500,
            owner_id: 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
            owner_email: 'alice@example.com',
            owner_pubkey: 'pub',
            owner_pubkey_fingerprint: 'fp',
            shared_by_user_id: 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
            shared_by_email: 'alice@example.com'
          }
        ]
      })
    )
    vi.spyOn(meta, 'decrypt').mockResolvedValue({ name: 'in-flight.bin' })

    const store = storageStore()
    await store.find(KEYPAIR, SHARED_WITH_ME_DIR_ID)

    const row = store.items.find((r) => r.id === '22222222-2222-2222-2222-222222222222')
    expect(row).toBeTruthy()
    expect(row?.chunks).toEqual(10)
    expect(row?.chunks_stored).toEqual(3)
    expect(row?.finished_upload_at).toBeUndefined()
  })
})
