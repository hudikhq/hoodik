import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import * as sharesApi from '../../services/shares/api'
import { grantsStore } from '../../services/shares/grants'
import { store as sharesStore } from '../../services/shares'
import { store as linksStore } from '../../services/links'

import type { AppLink, AppShare, KeyPair } from '../../types'

const FILE_ID = '11111111-1111-1111-1111-111111111111'
const OTHER_FILE_ID = '22222222-2222-2222-2222-222222222222'
const KEYPAIR: KeyPair = { input: 'priv', publicKey: 'pub' } as KeyPair

function sampleShare(overrides: Partial<AppShare> = {}): AppShare {
  return {
    file_id: FILE_ID,
    recipient_id: 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb',
    recipient_email: 'bob@example.com',
    recipient_pubkey_fingerprint: 'fp-bob',
    share_role: 'reader',
    created_at: 1_700_001_000,
    shared_at: 1_700_001_000,
    shared_by_user_id: 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
    shared_by_email: 'alice@example.com',
    ...overrides
  }
}

function sampleLink(overrides: Partial<AppLink> = {}): AppLink {
  return {
    id: 'cccccccc-cccc-cccc-cccc-cccccccccccc',
    file_id: FILE_ID,
    owner_id: 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
    owner_email: 'alice@example.com',
    owner_pubkey: 'owner-pub',
    file_size: 1024,
    file_mime: 'image/png',
    signature: 'sig',
    downloads: 0,
    encrypted_name: 'enc-name',
    encrypted_link_key: 'enc-key',
    created_at: 1_700_002_000,
    file_modified_at: 1_700_002_000,
    name: 'image.png',
    link_key: new Uint8Array([1, 2, 3]),
    link_key_hex: '010203',
    ...overrides
  } as AppLink
}

beforeEach(() => {
  setActivePinia(createPinia())
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('grants store', () => {
  it('grants_unloaded_file_returns_empty_array', () => {
    const store = grantsStore()
    expect(store.grants(FILE_ID).value).toEqual([])
    expect(store.isLoading(FILE_ID)).toBe(false)
  })

  it('grants_load_populates_user_and_link_kinds_for_file', async () => {
    const shareRow = sampleShare()
    const linkRow = sampleLink()
    vi.spyOn(sharesApi, 'getShareRecipients').mockResolvedValue([shareRow])
    const linksApi = linksStore()
    vi.spyOn(linksApi, 'find').mockImplementation(async () => {
      linksApi.upsertItem(linkRow)
    })

    const store = grantsStore()
    await store.loadGrants(FILE_ID, KEYPAIR)

    const list = store.grants(FILE_ID).value
    expect(list).toHaveLength(2)
    const user = list.find((g) => g.kind === 'user')
    const link = list.find((g) => g.kind === 'link')
    expect(user).toBeTruthy()
    expect(link).toBeTruthy()
    if (user && user.kind === 'user') {
      expect(user.recipient_id).toEqual(shareRow.recipient_id)
      expect(user.user.recipient_email).toEqual('bob@example.com')
    }
    if (link && link.kind === 'link') {
      expect(link.link_id).toEqual(linkRow.id)
      expect(link.link.file_id).toEqual(FILE_ID)
    }
  })

  it('grants_filter_by_file_id_excludes_unrelated_rows', async () => {
    const ours = sampleShare()
    const otherLink = sampleLink({ id: 'dddddddd-dddd-dddd-dddd-dddddddddddd', file_id: OTHER_FILE_ID })
    const ourLink = sampleLink()
    vi.spyOn(sharesApi, 'getShareRecipients').mockResolvedValue([ours])
    const linksApi = linksStore()
    vi.spyOn(linksApi, 'find').mockImplementation(async () => {
      linksApi.upsertItem(ourLink)
      linksApi.upsertItem(otherLink)
    })

    const store = grantsStore()
    await store.loadGrants(FILE_ID, KEYPAIR)

    const list = store.grants(FILE_ID).value
    const linkRows = list.filter((g) => g.kind === 'link')
    expect(linkRows).toHaveLength(1)
    if (linkRows[0].kind === 'link') {
      expect(linkRows[0].file_id).toEqual(FILE_ID)
    }
  })

  it('grants_revoke_user_delegates_and_drops_row', async () => {
    const shareRow = sampleShare()
    vi.spyOn(sharesApi, 'getShareRecipients').mockResolvedValue([shareRow])
    const linksApi = linksStore()
    vi.spyOn(linksApi, 'find').mockResolvedValue(undefined)
    const revokeSpy = vi.spyOn(sharesApi, 'revokeShare').mockResolvedValue(undefined)

    const store = grantsStore()
    await store.loadGrants(FILE_ID, KEYPAIR)
    expect(store.grants(FILE_ID).value.filter((g) => g.kind === 'user')).toHaveLength(1)

    await store.revokeGrant(FILE_ID, shareRow.recipient_id, {
      event_signature: 'sig',
      timestamp: 1
    })

    expect(revokeSpy).toHaveBeenCalledWith(
      FILE_ID,
      shareRow.recipient_id,
      expect.objectContaining({ event_signature: 'sig' })
    )
    expect(store.grants(FILE_ID).value.filter((g) => g.kind === 'user')).toHaveLength(0)
  })

  it('grants_revoke_link_delegates_to_links_store_and_drops_row', async () => {
    const linkRow = sampleLink()
    vi.spyOn(sharesApi, 'getShareRecipients').mockResolvedValue([])
    const linksApi = linksStore()
    vi.spyOn(linksApi, 'find').mockImplementation(async () => {
      linksApi.upsertItem(linkRow)
    })
    const removeSpy = vi.spyOn(linksApi, 'remove').mockImplementation(async (id: string) => {
      linksApi.removeItem(id)
    })

    const store = grantsStore()
    await store.loadGrants(FILE_ID, KEYPAIR)
    expect(store.grants(FILE_ID).value.filter((g) => g.kind === 'link')).toHaveLength(1)

    await store.revokeLink(linkRow.id)

    expect(removeSpy).toHaveBeenCalledWith(linkRow.id)
    expect(store.grants(FILE_ID).value.filter((g) => g.kind === 'link')).toHaveLength(0)
  })

  it('grants_create_share_via_underlying_store_appears_in_list', async () => {
    const shares = sharesStore()
    const linksApi = linksStore()
    vi.spyOn(linksApi, 'find').mockResolvedValue(undefined)
    vi.spyOn(sharesApi, 'getShareRecipients').mockResolvedValue([])
    vi.spyOn(sharesApi, 'createShare').mockResolvedValue({
      shares: [sampleShare({ share_role: 'editor' })]
    })

    const store = grantsStore()
    await store.loadGrants(FILE_ID, KEYPAIR)
    expect(store.grants(FILE_ID).value).toHaveLength(0)

    await shares.createShare({
      payload_der: '',
      signature: '',
      entries: [],
      event_signature: ''
    })

    const list = store.grants(FILE_ID).value
    expect(list).toHaveLength(1)
    expect(list[0].kind).toEqual('user')
    if (list[0].kind === 'user') {
      expect(list[0].user.share_role).toEqual('editor')
    }
  })

  it('grants_loadGrants_failure_records_error_state', async () => {
    vi.spyOn(sharesApi, 'getShareRecipients').mockRejectedValue(new Error('boom'))
    const linksApi = linksStore()
    vi.spyOn(linksApi, 'find').mockResolvedValue(undefined)

    const store = grantsStore()
    await expect(store.loadGrants(FILE_ID, KEYPAIR)).rejects.toThrow('boom')
    expect(store.errorOf(FILE_ID)).toEqual('boom')
    expect(store.stateOf(FILE_ID).state).toEqual('error')
  })
})
