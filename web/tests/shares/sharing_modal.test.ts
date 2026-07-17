import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'

import SharingModal from '../../src/components/shares/SharingModal.vue'
import { store as loginStore } from '../../services/auth/login'
import { store as sharesStore } from '../../services/shares'
import { store as linksStore } from '../../services/links'
import * as sharesApi from '../../services/shares/api'
import * as shareCrypto from '../../services/shares/crypto'

import type { AppFile, AppLink, AppShare, KeyPair } from '../../types'

const OWNER_ID = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'
const BOB_ID = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb'
const FILE_ID = '11111111-1111-1111-1111-111111111111'

const OWNED_FILE: AppFile = {
  id: FILE_ID,
  user_id: OWNER_ID,
  is_owner: true,
  name: 'doc.pdf',
  name_hash: '',
  mime: 'application/pdf',
  chunks: 0,
  file_id: null,
  file_modified_at: 0,
  created_at: 0,
  is_new: false,
  editable: false,
  active_version: 1,
  encrypted_key: '',
  encrypted_name: '',
  cipher: 'aegis128l'
} as unknown as AppFile

const READER_FILE: AppFile = {
  ...OWNED_FILE,
  is_owner: false,
  share_role: 'reader'
} as unknown as AppFile

function sampleShare(overrides: Partial<AppShare> = {}): AppShare {
  return {
    file_id: FILE_ID,
    recipient_id: BOB_ID,
    recipient_email: 'bob@example.com',
    recipient_pubkey_fingerprint: 'a'.repeat(64),
    share_role: 'reader',
    created_at: 1_700_001_000,
    shared_at: 1_700_001_000,
    shared_by_user_id: OWNER_ID,
    shared_by_email: 'alice@example.com',
    ...overrides
  }
}

function sampleLink(overrides: Partial<AppLink> = {}): AppLink {
  return {
    id: 'cccccccc-cccc-cccc-cccc-cccccccccccc',
    file_id: FILE_ID,
    owner_id: OWNER_ID,
    owner_email: 'alice@example.com',
    owner_pubkey: 'pub',
    file_size: 1024,
    file_mime: 'application/pdf',
    signature: 'sig',
    downloads: 0,
    encrypted_name: '',
    encrypted_link_key: '',
    created_at: 1_700_003_000,
    file_modified_at: 1_700_003_000,
    name: 'doc.pdf',
    link_key: new Uint8Array([1, 2, 3]),
    link_key_hex: '010203',
    ...overrides
  } as AppLink
}

function setupAuth(): void {
  const login = loginStore()
  login.set({
    user: { id: OWNER_ID, email: 'alice@example.com', pubkey: '', fingerprint: '' },
    session: {}
  } as unknown as Parameters<typeof login.set>[0])
}

interface MountOptions {
  file?: AppFile | null
  initialTab?: 'people' | 'link'
  recipients?: AppShare[]
  links?: AppLink[]
}

function mountModal(options: MountOptions = {}): ReturnType<typeof mount> {
  const file = options.file ?? OWNED_FILE
  const recipients = options.recipients ?? []
  const seedLinks = options.links ?? []

  vi.spyOn(sharesApi, 'getShareRecipients').mockResolvedValue(recipients)
  vi.spyOn(sharesApi, 'revokeShare').mockResolvedValue(undefined)
  vi.spyOn(sharesApi, 'listGroups').mockResolvedValue({ owned: [], member_of: [] })
  // The revoke path signs an AuditEventSigInputV1; the keypair we hand
  // the modal is a stub, so we mock the signer to a deterministic value
  // rather than wire a real RSA key just for the click flow.
  vi.spyOn(shareCrypto, 'signAuditEvent').mockResolvedValue('test-signature')

  const links = linksStore()
  vi.spyOn(links, 'find').mockImplementation(async () => {
    for (const link of seedLinks) links.upsertItem(link)
  })

  return mount(SharingModal, {
    props: {
      file,
      authenticatedUserId: OWNER_ID,
      keypair: { input: 'priv', publicKey: 'pub' } as unknown as KeyPair,
      storage: { items: [] } as unknown as Parameters<typeof mount>[1] extends never
        ? never
        : Parameters<typeof mount>[1],
      links: links as unknown as Parameters<typeof mount>[1] extends never
        ? never
        : Parameters<typeof mount>[1],
      initialTab: options.initialTab
    } as unknown as Record<string, unknown>
  })
}

beforeEach(() => {
  setActivePinia(createPinia())
  if (typeof localStorage !== 'undefined') localStorage.clear()
  setupAuth()
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('SharingModal', () => {
  it('renders both tabs with their grant counts after loading', async () => {
    const wrapper = mountModal({
      recipients: [sampleShare()],
      links: [sampleLink()]
    })
    await flushPromises()

    const peopleTab = wrapper.get('[data-testid="sharing-modal-tab-people"]')
    const linkTab = wrapper.get('[data-testid="sharing-modal-tab-link"]')
    expect(peopleTab.text()).toContain('People')
    expect(peopleTab.text()).toContain('1')
    expect(linkTab.text()).toContain('Public link')
    expect(linkTab.text()).toContain('1')
  })

  it('switches the body when the user clicks the Link tab', async () => {
    const wrapper = mountModal({
      recipients: [sampleShare()],
      links: [sampleLink()]
    })
    await flushPromises()

    expect(wrapper.find('[data-testid="sharing-modal-people-list"]').exists()).toBe(true)

    await wrapper.get('[data-testid="sharing-modal-tab-link"]').trigger('click')

    expect(wrapper.find('[data-testid="sharing-modal-people-list"]').exists()).toBe(false)
    // The Link tab renders the link manage view (no people-list test id).
    expect(wrapper.text()).toContain('Public link for a file'.split(' ')[0])
  })

  it('revoke on a user grant calls the API and removes the row', async () => {
    const wrapper = mountModal({ recipients: [sampleShare()] })
    await flushPromises()

    const row = wrapper.find(`[data-testid="sharing-modal-people-row-${BOB_ID}"]`)
    expect(row.exists()).toBe(true)
    // BaseButtonConfirm renders one trigger button; after the first
    // click it swaps to a confirm/cancel pair. Find the BaseButtonConfirm
    // root by its testid and walk the rendered buttons to fire both
    // clicks. The component itself emits `confirm` which the modal
    // wires to `revokeUser`.
    const revokeRoot = row.findComponent({ name: 'BaseButtonConfirm' })
    expect(revokeRoot.exists()).toBe(true)
    await revokeRoot.find('button').trigger('click')
    await flushPromises()
    await revokeRoot.findAll('button')[0].trigger('click')
    await flushPromises()
    // Cryptographic signing can take an extra tick on the JSDOM side.
    await flushPromises()

    expect(sharesApi.revokeShare).toHaveBeenCalledWith(
      FILE_ID,
      BOB_ID,
      expect.objectContaining({ event_signature: expect.any(String) })
    )
    const shares = sharesStore()
    expect(shares.outgoingByFile[FILE_ID] ?? []).toHaveLength(0)
  })

  it('Reader sees the recipient list but cannot mutate it', async () => {
    const wrapper = mountModal({
      file: READER_FILE,
      recipients: [sampleShare()]
    })
    await flushPromises()

    const banner = wrapper.find('[data-testid="sharing-modal-readonly-banner"]')
    expect(banner.exists()).toBe(true)
    expect(banner.text().length).toBeGreaterThan(0)
    // Sharing a file shares the roster — Reader sees Bob's row even
    // though they cannot change anything.
    expect(wrapper.find(`[data-testid="sharing-modal-people-row-${BOB_ID}"]`).exists()).toBe(
      true
    )
    const submit = wrapper.find('[data-testid="share-dialog-submit"]')
    expect(submit.exists()).toBe(true)
    expect((submit.element as HTMLButtonElement).disabled).toBe(true)
    expect(
      wrapper.find(`[data-testid="sharing-modal-change-role-${BOB_ID}"]`).exists()
    ).toBe(false)
  })

  it('opens directly on the Link tab when initialTab=link', async () => {
    const wrapper = mountModal({
      initialTab: 'link',
      links: [sampleLink()]
    })
    await flushPromises()

    const linkTabButton = wrapper.get('[data-testid="sharing-modal-tab-link"]')
    expect(linkTabButton.classes().some((c) => c.includes('redish-500'))).toBe(true)
    expect(wrapper.find('[data-testid="sharing-modal-people-list"]').exists()).toBe(false)
  })

  it('owned folder surfaces the add-recipient form alongside the members list', async () => {
    // Folder shares route through the signed members-list endpoint, which
    // `FolderMembersView` is built around. The view shows existing members
    // but has no add-recipient surface of its own — without the modal
    // mounting `SharingPeopleAdd` next to it, owned folders end up with a
    // read-only "who has access" view and no way to grant more.
    const OWNED_FOLDER: AppFile = {
      ...OWNED_FILE,
      mime: 'dir'
    } as unknown as AppFile
    vi.spyOn(sharesApi, 'getFolderMembers').mockResolvedValue({
      folder_id: FILE_ID,
      folder_owner_id: OWNER_ID,
      folder_owner_pubkey_fingerprint: '',
      signature_algorithm: 'rsa-pss-sha256',
      members: [],
      members_signed_at: null,
      members_list_signature: null,
      members_list_signed_by_user_id: null
    })
    const wrapper = mountModal({ file: OWNED_FOLDER })
    await flushPromises()

    expect(wrapper.find('[data-testid="sharing-modal-folder-members"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="share-dialog-submit"]').exists()).toBe(true)
  })

  it('hides the entire tab strip on folders', async () => {
    // Folder public-link sharing is out of scope for v1; with only one
    // surface to show, the tab strip collapses and the body renders
    // directly. An `initialTab` of `link` falls back to People.
    const OWNED_FOLDER: AppFile = {
      ...OWNED_FILE,
      mime: 'dir'
    } as unknown as AppFile
    vi.spyOn(sharesApi, 'getFolderMembers').mockResolvedValue({
      folder_id: FILE_ID,
      folder_owner_id: OWNER_ID,
      folder_owner_pubkey_fingerprint: '',
      signature_algorithm: 'rsa-pss-sha256',
      members: [],
      members_signed_at: null,
      members_list_signature: null,
      members_list_signed_by_user_id: null
    })
    const wrapper = mountModal({ file: OWNED_FOLDER, initialTab: 'link' })
    await flushPromises()

    expect(wrapper.find('[data-testid="sharing-modal-tab-people"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="sharing-modal-tab-link"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="share-dialog-submit"]').exists()).toBe(true)
  })

  it('keeps both tabs on owned files', async () => {
    const wrapper = mountModal({ recipients: [sampleShare()], links: [sampleLink()] })
    await flushPromises()

    expect(wrapper.find('[data-testid="sharing-modal-tab-people"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="sharing-modal-tab-link"]').exists()).toBe(true)
  })

  it('hides the Public link tab from recipients on shared files', async () => {
    // Public links are owner-side. The recipient can't
    // decrypt the link key, so surfacing the tab to them leaks no
    // information and adds confusion. With only one surface to show,
    // the tab strip collapses.
    const wrapper = mountModal({
      file: READER_FILE,
      recipients: [sampleShare()],
      initialTab: 'link'
    })
    await flushPromises()

    expect(wrapper.find('[data-testid="sharing-modal-tab-link"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="sharing-modal-tab-people"]').exists()).toBe(false)
    // initialTab=link falls back to People when the Link tab isn't visible.
    expect(wrapper.find('[data-testid="sharing-modal-people-list"]').exists()).toBe(true)
  })
})
