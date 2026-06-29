import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { flushPromises, mount } from '@vue/test-utils'

import DirectoryTree from '../../src/components/files/browser/DirectoryTree.vue'
import * as meta from '../../services/storage/meta'
import * as sharesApi from '../../services/shares/api'
import { store as storageStore } from '../../services/storage'
import { classifyMove } from '../../services/storage/moveInto'

import type { AppFile, IncomingShare, IncomingSharePage, KeyPair, ShareRole } from '../../types'

const KEYPAIR: KeyPair = { input: 'priv', publicKey: 'pub' } as KeyPair

const EDITOR_FOLDER_ID = '11111111-1111-1111-1111-111111111111'
const READER_FOLDER_ID = '22222222-2222-2222-2222-222222222222'
const COOWNER_FOLDER_ID = '33333333-3333-3333-3333-333333333333'
const SUBFOLDER_ID = '44444444-4444-4444-4444-444444444444'

function incomingFolder(fileId: string, role: ShareRole, mime = 'dir'): IncomingShare {
  return {
    file_id: fileId,
    mime,
    encrypted_name: `enc-${fileId}`,
    cipher: 'aegis128l',
    editable: false,
    share_role: role,
    encrypted_key: `wrap-${fileId}`,
    created_at: 1_700_000_000,
    shared_at: 1_700_000_500,
    owner_id: 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
    owner_email: 'alice@example.com',
    owner_pubkey: 'pub',
    owner_pubkey_fingerprint: 'fp',
    shared_by_user_id: 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
    shared_by_email: 'alice@example.com'
  }
}

function page(items: IncomingShare[]): IncomingSharePage {
  return { items, total: items.length, limit: 50, offset: 0 }
}

function mountPicker() {
  const Storage = storageStore()
  const captured: (AppFile | undefined)[] = []
  const wrapper = mount(DirectoryTree, {
    props: { keypair: KEYPAIR, Storage, load: true },
    attrs: { onSelect: (file?: AppFile) => captured.push(file) }
  })
  return { wrapper, Storage, captured }
}

beforeEach(() => {
  setActivePinia(createPinia())
  // Owned-drive listing is empty in these tests; the focus is the shared branch.
  vi.spyOn(meta, 'find').mockResolvedValue({ children: [], parents: [] })
  vi.spyOn(meta, 'decrypt').mockImplementation(async (item) => {
    const wrap = (item as { encrypted_key?: string }).encrypted_key
    return { name: `name-${wrap}` }
  })
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('DirectoryTree move picker: Shared with me branch', () => {
  it('renders the branch and lists only writable shared folders', async () => {
    vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(
      page([
        incomingFolder(EDITOR_FOLDER_ID, 'editor'),
        incomingFolder(READER_FOLDER_ID, 'reader'),
        incomingFolder(COOWNER_FOLDER_ID, 'co-owner')
      ])
    )

    const { wrapper } = mountPicker()
    await flushPromises()

    expect(wrapper.find('[data-testid="directory-tree-shared-with-me"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('Shared with me')
    expect(wrapper.text()).toContain(`name-wrap-${EDITOR_FOLDER_ID}`)
    expect(wrapper.text()).toContain(`name-wrap-${COOWNER_FOLDER_ID}`)
    // Reader-only folder is not a valid move destination — never listed.
    expect(wrapper.text()).not.toContain(`name-wrap-${READER_FOLDER_ID}`)
  })

  it('omits the branch entirely when no writable share exists', async () => {
    vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(
      page([incomingFolder(READER_FOLDER_ID, 'reader')])
    )

    const { wrapper } = mountPicker()
    await flushPromises()

    expect(wrapper.find('[data-testid="directory-tree-shared-with-me"]').exists()).toBe(false)
    expect(wrapper.text()).not.toContain('Shared with me')
  })

  it('emits a shared AppFile that classifies as move-into-shared', async () => {
    vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(
      page([incomingFolder(EDITOR_FOLDER_ID, 'editor')])
    )

    const { wrapper, captured } = mountPicker()
    await flushPromises()

    // Scope to the shared folder's own node so we don't click the owned
    // "Root" Move button (which emits select(undefined)).
    const sharedNode = wrapper
      .findAllComponents(DirectoryTree)
      .find((c) => c.props('shared') === true && c.props('parent')?.id === EDITOR_FOLDER_ID)
    expect(sharedNode).toBeTruthy()

    // Two-stage confirm button: first click arms, second click confirms.
    const armButton = sharedNode!.findAll('button').find((b) => b.text().includes('Move'))
    expect(armButton).toBeTruthy()
    await armButton!.trigger('click')
    await flushPromises()
    const confirmButton = sharedNode!
      .findAll('button')
      .find((b) => b.text().includes('Confirm'))
    expect(confirmButton).toBeTruthy()
    await confirmButton!.trigger('click')
    await flushPromises()

    expect(captured).toHaveLength(1)
    const folder = captured[0] as AppFile
    expect(folder.id).toBe(EDITOR_FOLDER_ID)
    expect(folder.is_owner).toBe(false)
    expect(folder.share_role).toBe('editor')

    const decision = classifyMove({
      sources: [
        {
          id: 'owned-file',
          is_owner: true,
          mime: 'text/plain'
        } as AppFile
      ],
      destination: folder,
      sourceParent: null,
      sharingEnabled: true
    })
    expect(decision).toMatchObject({
      kind: 'into-shared',
      destinationFolderId: EDITOR_FOLDER_ID
    })
  })

  it('navigates into a writable shared folder using the recipient-aware listing', async () => {
    vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(
      page([incomingFolder(EDITOR_FOLDER_ID, 'editor')])
    )
    const findSpy = meta.find as unknown as ReturnType<typeof vi.fn>
    findSpy.mockImplementation(async (params: { dir_id?: string; is_owner?: boolean }) => {
      if (params?.dir_id === EDITOR_FOLDER_ID) {
        // Server returns the recipient's child rows: is_owner=false, the
        // recipient's role, and the child's own members_signed_at.
        return {
          children: [
            {
              id: SUBFOLDER_ID,
              file_id: EDITOR_FOLDER_ID,
              user_id: 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
              is_owner: false,
              share_role: 'editor',
              members_signed_at: 1_700_000_900,
              mime: 'dir',
              name_hash: 'h',
              chunks: 0,
              file_modified_at: 0,
              created_at: 0,
              is_new: false,
              editable: false,
              active_version: 1,
              encrypted_key: `wrap-${SUBFOLDER_ID}`,
              encrypted_name: 'enc',
              cipher: 'aegis128l'
            }
          ],
          parents: []
        }
      }
      return { children: [], parents: [] }
    })

    const { wrapper } = mountPicker()
    await flushPromises()

    const folderRow = wrapper
      .findAll('.cursor-pointer')
      .find((el) => el.text().includes(`name-wrap-${EDITOR_FOLDER_ID}`))
    expect(folderRow).toBeTruthy()
    await folderRow!.trigger('click')
    await flushPromises()

    // The recipient-aware listing call omits is_owner so the server returns
    // the child rows the caller has a key for inside someone else's folder.
    expect(findSpy).toHaveBeenCalledWith(
      expect.objectContaining({ dir_id: EDITOR_FOLDER_ID, dirs_only: true })
    )
    const sharedCall = findSpy.mock.calls.find(
      (call) => (call[0] as { dir_id?: string })?.dir_id === EDITOR_FOLDER_ID
    )
    expect((sharedCall![0] as { is_owner?: boolean }).is_owner).toBeUndefined()
    expect(wrapper.text()).toContain(`name-wrap-${SUBFOLDER_ID}`)
  })
})
