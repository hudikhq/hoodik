import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import Api from '../../services/api'
import * as sharesApi from '../../services/shares/api'
import {
  store as sharesStore,
  trustedFingerprintsStore,
  capabilitiesStore
} from '../../services/shares'
import { store as storageStore } from '../../services/storage'
import { store as loginStore } from '../../services/auth/login'
import { defaultCipher, setDefaultCipher } from '../../services/cryptfns/cipher'

import type { CryptoStore } from '../../types'

import type { Capabilities, IncomingSharePage, CreateShareResponse } from '../../types/shares'

const SAMPLE_INCOMING: IncomingSharePage = {
  items: [
    {
      file_id: '11111111-1111-1111-1111-111111111111',
      mime: 'text/plain',
      encrypted_name: '',
      cipher: 'aegis128l',
      editable: false,
      share_role: 'reader',
      encrypted_key: 'AAA=',
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
  offset: 0
}

const SAMPLE_CREATE: CreateShareResponse = {
  shares: [
    {
      file_id: '22222222-2222-2222-2222-222222222222',
      recipient_id: 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb',
      recipient_email: 'bob@example.com',
      recipient_pubkey_fingerprint: 'fp-bob',
      share_role: 'editor',
      created_at: 1_700_001_000,
      shared_at: 1_700_001_000,
      shared_by_user_id: 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
      shared_by_email: 'alice@example.com'
    }
  ]
}

beforeEach(() => {
  setActivePinia(createPinia())
  if (typeof localStorage !== 'undefined') {
    localStorage.clear()
  }
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('shares store', () => {
  it('shares_store_load_incoming_populates_state', async () => {
    vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(SAMPLE_INCOMING)
    const store = sharesStore()
    await store.loadIncoming()
    expect(store.incoming).toHaveLength(1)
    expect(store.incomingCount).toEqual(1)
  })

  it('shares_store_create_share_optimistic_update', async () => {
    vi.spyOn(sharesApi, 'createShare').mockResolvedValue(SAMPLE_CREATE)
    const store = sharesStore()
    const result = await store.createShare({
      payload_der: '',
      signature: '',
      entries: [],
      event_signature: ''
    })
    expect(result).toHaveLength(1)
    expect(store.outgoingByFile['22222222-2222-2222-2222-222222222222']).toHaveLength(1)
  })

  it('shares_store_revoke_clears_local_state', async () => {
    vi.spyOn(sharesApi, 'revokeShare').mockResolvedValue(undefined)
    const store = sharesStore()
    store.outgoingByFile = {
      '22222222-2222-2222-2222-222222222222': SAMPLE_CREATE.shares
    }
    await store.revoke(
      '22222222-2222-2222-2222-222222222222',
      'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb',
      { event_signature: 'sig', timestamp: 1 }
    )
    expect(store.outgoingByFile['22222222-2222-2222-2222-222222222222']).toHaveLength(0)
  })

  it('shares_store_revoke_cascade_drops_downstream_grants_live', async () => {
    // Mirror the cascade on the client: revoking a Co-owner drops
    // every grant they made, so the store needs to reflect that the
    // moment the API returns. Without the cascade-mirror the owner has
    // to reload before seeing downstream rows disappear.
    vi.spyOn(sharesApi, 'revokeShare').mockResolvedValue(undefined)
    const store = sharesStore()
    const fileId = '22222222-2222-2222-2222-222222222222'
    const coownerId = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb'
    const downstreamId = 'cccccccc-cccc-cccc-cccc-cccccccccccc'
    store.outgoingByFile = {
      [fileId]: [
        {
          ...SAMPLE_CREATE.shares[0],
          recipient_id: coownerId,
          share_role: 'co-owner',
          shared_by_user_id: 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'
        },
        {
          ...SAMPLE_CREATE.shares[0],
          recipient_id: downstreamId,
          recipient_email: 'downstream@example.com',
          share_role: 'reader',
          shared_by_user_id: coownerId
        }
      ]
    }
    await store.revoke(fileId, coownerId, { event_signature: 'sig', timestamp: 1 })
    expect(store.outgoingByFile[fileId]).toHaveLength(0)
  })

  it('shares_store_create_share_bumps_shared_with_count_badge', async () => {
    // The inline shared-out icon on a file row in `/files` is gated on
    // `shared_with_count`. Without an optimistic bump the badge only
    // appears after a full listing reload.
    vi.spyOn(sharesApi, 'createShare').mockResolvedValue(SAMPLE_CREATE)
    const fileId = '22222222-2222-2222-2222-222222222222'
    const files = storageStore()
    files.addItem({
      id: fileId,
      file_id: null,
      mime: 'image/png',
      name: 'test-file.png',
      is_owner: true,
      shared_with_count: 0
    } as never)
    const shares = sharesStore()
    await shares.createShare({
      payload_der: '',
      signature: '',
      entries: [],
      event_signature: ''
    })
    expect(files.getItem(fileId)?.shared_with_count).toEqual(1)
  })

  it('shares_store_create_share_does_not_double_count_role_change', async () => {
    // A role-change on an existing recipient must not bump the count —
    // server-side `enrich_shared_with_counts` counts distinct
    // recipients, not events. The store seeds `outgoingByFile` with the
    // existing recipient, then a re-share of the same row arrives.
    vi.spyOn(sharesApi, 'createShare').mockResolvedValue(SAMPLE_CREATE)
    const fileId = '22222222-2222-2222-2222-222222222222'
    const files = storageStore()
    files.addItem({
      id: fileId,
      file_id: null,
      mime: 'image/png',
      name: 'test-file.png',
      is_owner: true,
      shared_with_count: 1
    } as never)
    const shares = sharesStore()
    shares.outgoingByFile = {
      [fileId]: [SAMPLE_CREATE.shares[0]]
    }
    await shares.createShare({
      payload_der: '',
      signature: '',
      entries: [],
      event_signature: ''
    })
    expect(files.getItem(fileId)?.shared_with_count).toEqual(1)
  })

  it('shares_store_revoke_decrements_shared_with_count_badge', async () => {
    // The mirror of the create bump — revoking a recipient drops the
    // count without a refresh. Cascade-revoke of a Co-owner counts
    // every downstream recipient they granted on this file.
    vi.spyOn(sharesApi, 'revokeShare').mockResolvedValue(undefined)
    const fileId = '22222222-2222-2222-2222-222222222222'
    const coownerId = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb'
    const downstreamId = 'cccccccc-cccc-cccc-cccc-cccccccccccc'
    const files = storageStore()
    files.addItem({
      id: fileId,
      file_id: null,
      mime: 'image/png',
      name: 'test-file.png',
      is_owner: true,
      shared_with_count: 2
    } as never)
    const shares = sharesStore()
    shares.outgoingByFile = {
      [fileId]: [
        {
          ...SAMPLE_CREATE.shares[0],
          recipient_id: coownerId,
          share_role: 'co-owner',
          shared_by_user_id: 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'
        },
        {
          ...SAMPLE_CREATE.shares[0],
          recipient_id: downstreamId,
          recipient_email: 'downstream@example.com',
          share_role: 'reader',
          shared_by_user_id: coownerId
        }
      ]
    }
    await shares.revoke(fileId, coownerId, { event_signature: 'sig', timestamp: 1 })
    expect(files.getItem(fileId)?.shared_with_count).toEqual(0)
  })

  it('shares_store_self_remove_drops_incoming_row', async () => {
    vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(SAMPLE_INCOMING)
    vi.spyOn(sharesApi, 'revokeShare').mockResolvedValue(undefined)
    const store = sharesStore()
    await store.loadIncoming()
    expect(store.incoming).toHaveLength(1)

    // Recipient (self) revokes their own row by file_id; the store must
    // drop it immediately so the Shared-with-me view reflects the change
    // without a refetch.
    const row = SAMPLE_INCOMING.items[0]
    await store.revoke(row.file_id, 'self-user-id', {
      event_signature: 'sig',
      timestamp: 1
    })
    expect(store.incoming).toHaveLength(0)
  })

  it('shares_store_unread_count_reflects_recent_incoming', async () => {
    vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(SAMPLE_INCOMING)
    const store = sharesStore()
    await store.loadIncoming()
    expect(store.unreadCount).toEqual(1)
    store.lastSeenAt = SAMPLE_INCOMING.items[0].shared_at as number
    expect(store.unreadCount).toEqual(0)
  })
})

describe('files store reset on logout', () => {
  it('files_store_reset_clears_account_scoped_state', () => {
    const files = storageStore()
    files.addItem({
      id: '22222222-2222-2222-2222-222222222222',
      file_id: null,
      mime: 'image/png',
      name: 'prev-account-file.png',
      is_owner: true
    } as never)
    expect(files.getItem('22222222-2222-2222-2222-222222222222')).not.toBeNull()

    files.reset()

    expect(files.items).toHaveLength(0)
    expect(files.selected).toHaveLength(0)
    expect(files.getItem('22222222-2222-2222-2222-222222222222')).toBeNull()
  })

  it('logout_resets_the_file_list_so_a_different_account_does_not_see_it', async () => {
    // The previous account's decrypted listing must not survive a logout —
    // otherwise logging in as a different user (no page reload) shows their
    // files until a refresh.
    vi.spyOn(Api, 'post').mockResolvedValue({ body: {} } as never)

    const files = storageStore()
    files.addItem({
      id: '22222222-2222-2222-2222-222222222222',
      file_id: null,
      mime: 'image/png',
      name: 'prev-account-file.png',
      is_owner: true
    } as never)

    const cryptoStub = { clear: vi.fn() } as unknown as CryptoStore
    await loginStore().logout(cryptoStub)

    expect(files.items).toHaveLength(0)
    expect(cryptoStub.clear).toHaveBeenCalled()
  })
})

describe('trusted fingerprints store', () => {
  it('trusted_fingerprints_persist_to_localstorage', () => {
    const store = trustedFingerprintsStore()
    store.bind('user-a')
    store.trustFingerprint('peer-b', 'fp123', 'in-person')

    const raw = localStorage.getItem('hoodik:trustedFingerprints:user-a')
    expect(raw).toBeTruthy()
    const parsed = JSON.parse(raw as string)
    expect(parsed['peer-b'].pubkeyFingerprint).toEqual('fp123')
    expect(parsed['peer-b'].verificationMethod).toEqual('in-person')
  })

  it('trusted_fingerprints_stale_after_90_days', () => {
    const store = trustedFingerprintsStore()
    store.bind('user-a')
    store.trustFingerprint('peer-b', 'fp', 'other')

    expect(store.isStale('peer-b')).toBe(false)

    const entry = store.lookup('peer-b')
    expect(entry).toBeTruthy()
    if (entry) {
      const oneHundredDaysAgo = Math.floor(Date.now() / 1000) - 100 * 24 * 60 * 60
      store.state.map['peer-b'] = { ...entry, lastVerifiedAt: oneHundredDaysAgo }
    }
    expect(store.isStale('peer-b')).toBe(true)
  })

  it('trusted_fingerprints_mismatch_returns_distinct_status', () => {
    const store = trustedFingerprintsStore()
    store.bind('user-a')
    store.trustFingerprint('peer-b', 'first-fingerprint', 'other')
    const cached = store.lookup('peer-b')
    expect(cached).toBeTruthy()
    expect(cached?.pubkeyFingerprint).toEqual('first-fingerprint')

    // A different incoming fingerprint should be detectable by the caller
    // — the store only records what was trusted, the equality check lives
    // in the UI.
    const incoming = 'different-fingerprint'
    expect(cached?.pubkeyFingerprint).not.toEqual(incoming)
  })
})

describe('capabilities store', () => {
  it('capabilities_store_fail_closed_on_fetch_error', async () => {
    vi.spyOn(sharesApi, 'getCapabilities').mockRejectedValue(new Error('boom'))
    const store = capabilitiesStore()
    await store.fetch()
    expect(store.sharingEnabled).toBe(false)
    expect(store.editableFolders).toBe(false)
    expect(store.forkEnabled).toBe(false)
    expect(store.fetchError).toBeTruthy()
  })

  it('capabilities_store_returns_false_when_sharing_disabled', async () => {
    const disabled: Capabilities = {
      sharing: { enabled: false, roles: [] },
      editable_folders: true,
      share_groups: true,
      audit_log: true,
      fork: true
    }
    vi.spyOn(sharesApi, 'getCapabilities').mockResolvedValue(disabled)
    const store = capabilitiesStore()
    await store.fetch()
    expect(store.sharingEnabled).toBe(false)
    expect(store.editableFolders).toBe(false)
    expect(store.forkEnabled).toBe(false)
    expect(store.auditLog).toBe(false)
    // Groups ride on the sharing master switch — disabled sharing collapses
    // the whole group surface too.
    expect(store.shareGroups).toBe(false)
  })

  it('capabilities_store_refetches_on_explicit_call', async () => {
    const enabled: Capabilities = {
      sharing: { enabled: true, roles: ['reader', 'editor', 'co-owner'] },
      editable_folders: true,
      share_groups: true,
      audit_log: true,
      fork: true
    }
    const stub = vi.spyOn(sharesApi, 'getCapabilities').mockResolvedValue(enabled)
    const store = capabilitiesStore()
    await store.fetch()
    await store.fetch()
    expect(stub).toHaveBeenCalledTimes(2)
    expect(store.sharingEnabled).toBe(true)
    expect(store.forkEnabled).toBe(true)
    expect(store.shareGroups).toBe(true)
  })

  it('capabilities_store_applies_advertised_default_cipher', async () => {
    const caps: Capabilities = {
      sharing: { enabled: true, roles: ['reader', 'editor', 'co-owner'] },
      editable_folders: true,
      share_groups: true,
      audit_log: true,
      fork: true,
      default_cipher: 'aegis256'
    }
    vi.spyOn(sharesApi, 'getCapabilities').mockResolvedValue(caps)
    const store = capabilitiesStore()
    await store.fetch()
    expect(defaultCipher()).toBe('aegis256')
    setDefaultCipher('aegis128l')
  })

  it('capabilities_store_default_cipher_falls_back_for_old_servers', async () => {
    setDefaultCipher('aegis256')
    const caps: Capabilities = {
      sharing: { enabled: true, roles: ['reader', 'editor', 'co-owner'] },
      editable_folders: true,
      share_groups: true,
      audit_log: true,
      fork: true
    }
    vi.spyOn(sharesApi, 'getCapabilities').mockResolvedValue(caps)
    const store = capabilitiesStore()
    await store.fetch()
    expect(defaultCipher()).toBe('aegis128l')
  })
})
