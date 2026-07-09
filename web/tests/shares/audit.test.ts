import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { flushPromises, mount } from '@vue/test-utils'
import { createRouter, createMemoryHistory } from 'vue-router'

import * as cryptfns from '../../services/cryptfns'
import * as sharesApi from '../../services/shares/api'
import * as shareCrypto from '../../services/shares/crypto'
import * as storageMeta from '../../services/storage/meta'
import { store as sharesStore } from '../../services/shares'
import ShareHubAudit from '../../src/views/shares/ShareHubAudit.vue'

import type {
  AuditUserRef,
  ShareEvent,
  ShareEventPage,
  ShareRole
} from '../../types/shares'

const SENDER_ID = '11111111-1111-1111-1111-111111111111'
const RECIPIENT_ID = '22222222-2222-2222-2222-222222222222'
const FILE_ID = '33333333-3333-3333-3333-333333333333'

function uuidToBytes(uuid: string): Uint8Array {
  return cryptfns.uint8.fromHex(uuid.replace(/-/g, ''))
}

const AUDIT_EVENT_V1_PREFIX = new TextEncoder().encode('hoodik-audit-v1\0')

async function buildRow(
  index: number,
  privateKey: string,
  prevHashBase64: string | null,
  override: Partial<ShareEvent> = {}
): Promise<ShareEvent> {
  const action = override.action ?? 'grant'
  const roleAfter = (override.share_role_after ?? 'editor') as ShareRole | null
  const created_at = override.created_at ?? 1_700_000_000 + index
  const senderId = override.sender_id ?? SENDER_ID
  const recipientId = override.recipient_id ?? RECIPIENT_ID
  const fileId = override.file_id ?? FILE_ID

  const wasm = await import('../../services/cryptfns/wasm')
  const der = wasm.audit_event_encode_v1(
    uuidToBytes(senderId),
    recipientId ? uuidToBytes(recipientId) : new Uint8Array(16),
    uuidToBytes(fileId),
    action,
    roleAfter === null ? 0xff : roleAfter === 'reader' ? 0 : roleAfter === 'editor' ? 1 : 2,
    BigInt(created_at)
  )
  if (!der) throw new Error('Failed to encode row DER')
  const prev = prevHashBase64 ? cryptfns.uint8.fromBase64(prevHashBase64) : new Uint8Array(32)
  const buffer = new Uint8Array(AUDIT_EVENT_V1_PREFIX.length + prev.length + der.length)
  buffer.set(AUDIT_EVENT_V1_PREFIX, 0)
  buffer.set(prev, AUDIT_EVENT_V1_PREFIX.length)
  buffer.set(der, AUDIT_EVENT_V1_PREFIX.length + prev.length)
  const hashHex = cryptfns.sha256.digest(buffer)
  const this_event_hash = cryptfns.uint8.toBase64(cryptfns.uint8.fromHex(hashHex))

  let signature: string | null = null
  if (override.sender_signature === null) {
    signature = null
  } else {
    const sigInput = shareCrypto.buildAuditEventSigInput({
      senderId,
      recipientId,
      fileId,
      action,
      shareRoleBefore: (override.share_role_before ?? null) as ShareRole | null,
      shareRoleAfter: roleAfter,
      timestamp: BigInt(created_at)
    })
    signature = await shareCrypto.signAuditEvent(sigInput, privateKey)
  }

  return {
    id: override.id ?? `event-${index}`,
    sender_id: senderId,
    recipient_id: recipientId,
    file_id: fileId,
    action,
    share_role_before: (override.share_role_before ?? null) as ShareRole | null,
    share_role_after: roleAfter,
    created_at,
    prev_event_hash: prevHashBase64,
    this_event_hash,
    sender_signature: signature,
    encrypted_name: override.encrypted_name ?? null,
    cipher: override.cipher ?? null,
    encrypted_key: override.encrypted_key ?? null
  }
}

function setupRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/share/audit', name: 'share-audit', component: ShareHubAudit }]
  })
}

beforeEach(() => {
  setActivePinia(createPinia())
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('audit log view', () => {
  it('audit_view_renders_events_newest_first', async () => {
    const kp = await cryptfns.rsa.generateKeyPair()
    const r1 = await buildRow(0, kp.input as string, null, { created_at: 1_700_000_001 })
    const r2 = await buildRow(1, kp.input as string, r1.this_event_hash, {
      created_at: 1_700_000_002
    })
    const users: Record<string, AuditUserRef> = {
      [SENDER_ID]: {
        id: SENDER_ID,
        email: 'alice@example.com',
        pubkey: kp.publicKey as string,
        fingerprint: 'fp'
      }
    }
    const page: ShareEventPage = {
      events: [r2, r1],
      users,
      total: 2,
      limit: 100,
      offset: 0
    }
    vi.spyOn(sharesApi, 'getShareEvents').mockResolvedValue(page)
    const router = setupRouter()
    router.push('/share/audit')
    await router.isReady()
    const wrapper = mount(ShareHubAudit, { global: { plugins: [router] } })
    await flushPromises()
    const rows = wrapper.findAll('[data-testid="share-hub-audit-list"] > li')
    expect(rows.length).toBe(2)
    expect(rows[0].text()).toContain('alice@example.com')
  })

  it('audit_chain_verification_passes_on_intact_chain', async () => {
    const kp = await cryptfns.rsa.generateKeyPair()
    const r1 = await buildRow(0, kp.input as string, null, { created_at: 1_700_000_001 })
    const r2 = await buildRow(1, kp.input as string, r1.this_event_hash, {
      created_at: 1_700_000_002
    })
    const events: ShareEvent[] = [r2, r1]
    const result = shareCrypto.verifyChain(events)
    expect(result.chainOk).toEqual([true, true])
    expect(result.firstBreakIndex).toBe(-1)
  })

  it('audit_chain_verifies_system_cascade_with_out_of_slice_predecessor', async () => {
    // The system bucket's per-row prev_event_hash often points at a
    // cascade row from a different folder that the current filter
    // (e.g. file_id=F) excluded. The walker must still mark the row
    // as verified when the self-hash recomputes correctly — refusing
    // to do so renders the chain-mismatch badge on every cascade.
    const cascadeFileId = '44444444-4444-4444-4444-444444444444'
    const previousCascadeHash = cryptfns.uint8.toBase64(new Uint8Array(32).fill(7))
    const wasm = await import('../../services/cryptfns/wasm')
    const der = wasm.audit_event_encode_v1(
      new Uint8Array(16),
      uuidToBytes(RECIPIENT_ID),
      uuidToBytes(cascadeFileId),
      'shared_by_co_owner_revoked',
      0xff,
      BigInt(1_700_000_100)
    )
    if (!der) throw new Error('failed to encode')
    const prev = cryptfns.uint8.fromBase64(previousCascadeHash)
    const buffer = new Uint8Array(AUDIT_EVENT_V1_PREFIX.length + prev.length + der.length)
    buffer.set(AUDIT_EVENT_V1_PREFIX, 0)
    buffer.set(prev, AUDIT_EVENT_V1_PREFIX.length)
    buffer.set(der, AUDIT_EVENT_V1_PREFIX.length + prev.length)
    const hashHex = cryptfns.sha256.digest(buffer)
    const cascadeRow: ShareEvent = {
      id: 'cascade-1',
      sender_id: null,
      recipient_id: RECIPIENT_ID,
      file_id: cascadeFileId,
      action: 'shared_by_co_owner_revoked',
      share_role_before: 'reader',
      share_role_after: null,
      created_at: 1_700_000_100,
      prev_event_hash: previousCascadeHash,
      this_event_hash: cryptfns.uint8.toBase64(cryptfns.uint8.fromHex(hashHex)),
      sender_signature: null,
      encrypted_name: null,
      cipher: null,
      encrypted_key: null
    }
    const result = shareCrypto.verifyChain([cascadeRow])
    expect(result.chainOk).toEqual([true])
    expect(result.firstBreakIndex).toBe(-1)
  })

  it('audit_chain_slice_aware_single_mid_bucket_gap_does_not_break', async () => {
    // A recipient's paged view sees grants targeting them but not an
    // intervening revoke of another user. A row whose `prev_event_hash`
    // points at that missing row is a page boundary, not a tamper: the
    // verifier must treat it as a fresh chain head and self-verify only.
    const kp = await cryptfns.rsa.generateKeyPair()
    const r1 = await buildRow(0, kp.input as string, null, { created_at: 1_700_000_001 })
    const r2 = await buildRow(1, kp.input as string, r1.this_event_hash, {
      created_at: 1_700_000_002
    })
    // r3's prev points at an OUT-OF-PAGE hash (simulating Bob's view
    // missing the intervening revoke-of-Carol row).
    const outOfPageHash = cryptfns.uint8.toBase64(new Uint8Array(32).fill(9))
    const r3 = await buildRow(2, kp.input as string, outOfPageHash, {
      created_at: 1_700_000_003
    })
    const r4 = await buildRow(3, kp.input as string, r3.this_event_hash, {
      created_at: 1_700_000_004
    })
    const result = shareCrypto.verifyChain([r4, r3, r2, r1])
    // Every row's self-hash recomputes — none of them are tampered.
    expect(result.chainOk).toEqual([true, true, true, true])
    expect(result.firstBreakIndex).toBe(-1)
  })

  it('audit_chain_slice_aware_multiple_mid_bucket_gaps_do_not_break', async () => {
    const kp = await cryptfns.rsa.generateKeyPair()
    const gap1 = cryptfns.uint8.toBase64(new Uint8Array(32).fill(11))
    const gap2 = cryptfns.uint8.toBase64(new Uint8Array(32).fill(13))
    const r1 = await buildRow(0, kp.input as string, null, { created_at: 1_700_000_001 })
    const r2 = await buildRow(1, kp.input as string, gap1, { created_at: 1_700_000_002 })
    const r3 = await buildRow(2, kp.input as string, r2.this_event_hash, {
      created_at: 1_700_000_003
    })
    const r4 = await buildRow(3, kp.input as string, gap2, { created_at: 1_700_000_004 })
    const result = shareCrypto.verifyChain([r4, r3, r2, r1])
    expect(result.chainOk).toEqual([true, true, true, true])
    expect(result.firstBreakIndex).toBe(-1)
  })

  it('audit_chain_slice_aware_gap_at_end_does_not_break', async () => {
    const kp = await cryptfns.rsa.generateKeyPair()
    const r1 = await buildRow(0, kp.input as string, null, { created_at: 1_700_000_001 })
    const r2 = await buildRow(1, kp.input as string, r1.this_event_hash, {
      created_at: 1_700_000_002
    })
    // The newest row in the bucket has an out-of-page predecessor.
    const outOfPageHash = cryptfns.uint8.toBase64(new Uint8Array(32).fill(17))
    const r3 = await buildRow(2, kp.input as string, outOfPageHash, {
      created_at: 1_700_000_003
    })
    const result = shareCrypto.verifyChain([r3, r2, r1])
    expect(result.chainOk).toEqual([true, true, true])
    expect(result.firstBreakIndex).toBe(-1)
  })

  it('audit_chain_slice_aware_entire_bucket_out_of_page_predecessors', async () => {
    // Pathological: every row's predecessor is outside the slice.
    // The verifier self-verifies each row's hash; none links to
    // another in-page row, so no link-check fires. All rows verify.
    const kp = await cryptfns.rsa.generateKeyPair()
    const gap1 = cryptfns.uint8.toBase64(new Uint8Array(32).fill(20))
    const gap2 = cryptfns.uint8.toBase64(new Uint8Array(32).fill(21))
    const gap3 = cryptfns.uint8.toBase64(new Uint8Array(32).fill(22))
    const r1 = await buildRow(0, kp.input as string, gap1, { created_at: 1_700_000_001 })
    const r2 = await buildRow(1, kp.input as string, gap2, { created_at: 1_700_000_002 })
    const r3 = await buildRow(2, kp.input as string, gap3, { created_at: 1_700_000_003 })
    const result = shareCrypto.verifyChain([r3, r2, r1])
    expect(result.chainOk).toEqual([true, true, true])
    expect(result.firstBreakIndex).toBe(-1)
  })

  it('audit_chain_tampered_self_hash_still_flags_under_slice_aware_walk', async () => {
    // The relaxation only affects link-checks for page-boundary
    // rows — self-hash tampers must still surface.
    const kp = await cryptfns.rsa.generateKeyPair()
    const r1 = await buildRow(0, kp.input as string, null, { created_at: 1_700_000_001 })
    const r2 = await buildRow(1, kp.input as string, r1.this_event_hash, {
      created_at: 1_700_000_002
    })
    const tamperedHash = cryptfns.uint8.toBase64(new Uint8Array(32).fill(99))
    const tampered: ShareEvent = { ...r2, this_event_hash: tamperedHash }
    const result = shareCrypto.verifyChain([tampered, r1])
    expect(result.chainOk[1]).toBe(true)
    expect(result.chainOk[0]).toBe(false)
    expect(result.firstBreakIndex).toBe(0)
  })

  it('audit_chain_break_shown_at_first_inconsistency', async () => {
    const kp = await cryptfns.rsa.generateKeyPair()
    const r1 = await buildRow(0, kp.input as string, null, { created_at: 1_700_000_001 })
    const r2 = await buildRow(1, kp.input as string, r1.this_event_hash, {
      created_at: 1_700_000_002
    })
    const tampered: ShareEvent = { ...r2, created_at: r2.created_at + 1 }
    // Newest-first as returned by /api/shares/events.
    const events: ShareEvent[] = [tampered, r1]
    const result = shareCrypto.verifyChain(events)
    // r1 (older) verifies first against the empty prev hash — OK.
    // tampered's stored `this_event_hash` no longer matches a recompute
    // over its mutated `created_at` — chain break at the newer row.
    expect(result.chainOk[1]).toBe(true)
    expect(result.chainOk[0]).toBe(false)
    expect(result.firstBreakIndex).toBe(0)
  })

  it('audit_signature_verification_passes_on_valid_sig', async () => {
    const kp = await cryptfns.rsa.generateKeyPair()
    const row = await buildRow(0, kp.input as string, null)
    expect(
      await shareCrypto.verifyEventSignature(row, { pubkey: kp.publicKey as string })
    ).toBe(true)
  })

  it('audit_signature_verification_fails_on_tampered_payload', async () => {
    const kp = await cryptfns.rsa.generateKeyPair()
    const row = await buildRow(0, kp.input as string, null)
    const tampered: ShareEvent = { ...row, share_role_after: 'reader' }
    expect(
      await shareCrypto.verifyEventSignature(tampered, { pubkey: kp.publicKey as string })
    ).toBe(false)
  })

  it('audit_null_signature_rows_show_system_badge', async () => {
    const kp = await cryptfns.rsa.generateKeyPair()
    const row = await buildRow(0, kp.input as string, null, {
      sender_signature: null,
      sender_id: null,
      action: 'shared_folder_evict'
    })
    const users: Record<string, AuditUserRef> = {}
    const page: ShareEventPage = {
      events: [row],
      users,
      total: 1,
      limit: 100,
      offset: 0
    }
    vi.spyOn(sharesApi, 'getShareEvents').mockResolvedValue(page)
    const router = setupRouter()
    router.push('/share/audit')
    await router.isReady()
    const wrapper = mount(ShareHubAudit, { global: { plugins: [router] } })
    await flushPromises()
    expect(wrapper.find(`[data-testid="share-hub-audit-row-${row.id}-system"]`).exists()).toBe(true)
  })

  it('audit_tampered_banner_fires_on_self_hash_mismatch', async () => {
    // Self-hash tampering must surface the banner even
    // when the row's signature is still intact (signature covers the
    // payload, not the hash bytes; flipping `this_event_hash` alone
    // would slip past a sig-only check).
    const kp = await cryptfns.rsa.generateKeyPair()
    const r1 = await buildRow(0, kp.input as string, null, { created_at: 1_700_000_001 })
    const r2 = await buildRow(1, kp.input as string, r1.this_event_hash, {
      created_at: 1_700_000_002
    })
    const tamperedHash = cryptfns.uint8.toBase64(new Uint8Array(32).fill(99))
    const tampered: ShareEvent = { ...r2, this_event_hash: tamperedHash }
    const users: Record<string, AuditUserRef> = {
      [SENDER_ID]: {
        id: SENDER_ID,
        email: 'alice@example.com',
        pubkey: kp.publicKey as string,
        fingerprint: 'fp'
      }
    }
    const page: ShareEventPage = {
      events: [tampered, r1],
      users,
      total: 2,
      limit: 100,
      offset: 0
    }
    vi.spyOn(sharesApi, 'getShareEvents').mockResolvedValue(page)
    const router = setupRouter()
    router.push('/share/audit')
    await router.isReady()
    const wrapper = mount(ShareHubAudit, { global: { plugins: [router] } })
    await flushPromises()
    expect(
      wrapper.find(`[data-testid="share-hub-audit-row-${tampered.id}-tampered-banner"]`).exists()
    ).toBe(true)
    expect(
      wrapper.find(`[data-testid="share-hub-audit-row-${tampered.id}-export"]`).exists()
    ).toBe(true)
    expect(
      wrapper.find(`[data-testid="share-hub-audit-row-${r1.id}-tampered-banner"]`).exists()
    ).toBe(false)
  })

  it('audit_tampered_banner_fires_on_signature_mismatch', async () => {
    // The row's signature is computed over a different recipient than
    // the one persisted on the row, so signature verification fails
    // against the named sender even though self-hash recompute passes
    // (the hash sees the persisted recipient bytes only via the chain
    // payload, not the signature payload, so the row's own bytes still
    // match).
    const kp = await cryptfns.rsa.generateKeyPair()
    const row = await buildRow(0, kp.input as string, null, { created_at: 1_700_000_001 })
    const wrongRecipient = '99999999-9999-9999-9999-999999999999'
    const tamperedSig = await shareCrypto.signAuditEvent(
      shareCrypto.buildAuditEventSigInput({
        senderId: SENDER_ID,
        recipientId: wrongRecipient,
        fileId: FILE_ID,
        action: 'grant',
        shareRoleBefore: null,
        shareRoleAfter: 'editor',
        timestamp: BigInt(row.created_at)
      }),
      kp.input as string
    )
    const tampered: ShareEvent = { ...row, sender_signature: tamperedSig }
    const users: Record<string, AuditUserRef> = {
      [SENDER_ID]: {
        id: SENDER_ID,
        email: 'alice@example.com',
        pubkey: kp.publicKey as string,
        fingerprint: 'fp'
      }
    }
    const page: ShareEventPage = {
      events: [tampered],
      users,
      total: 1,
      limit: 100,
      offset: 0
    }
    vi.spyOn(sharesApi, 'getShareEvents').mockResolvedValue(page)
    const router = setupRouter()
    router.push('/share/audit')
    await router.isReady()
    const wrapper = mount(ShareHubAudit, { global: { plugins: [router] } })
    await flushPromises()
    expect(
      wrapper.find(`[data-testid="share-hub-audit-row-${tampered.id}-tampered-banner"]`).exists()
    ).toBe(true)
  })

  it('audit_tampered_banner_fires_on_chain_link_break', async () => {
    // Two visible adjacent rows in the same bucket where the newer
    // row's `prev_event_hash` does NOT match the predecessor's
    // `this_event_hash` — and the predecessor IS visible in the page
    // (so it's not a page-boundary). The verifier classifies this as
    // `link-broken` and the banner fires.
    const kp = await cryptfns.rsa.generateKeyPair()
    const r1 = await buildRow(0, kp.input as string, null, { created_at: 1_700_000_001 })
    const r2 = await buildRow(1, kp.input as string, r1.this_event_hash, {
      created_at: 1_700_000_002
    })
    // r3 claims to follow r1 (already in-page) — the link mismatches r2.
    // Recompute r3's self-hash against the new (mismatched) prev so
    // the failure isolates the link-check.
    const wasm = await import('../../services/cryptfns/wasm')
    const der = wasm.audit_event_encode_v1(
      uuidToBytes(SENDER_ID),
      uuidToBytes(RECIPIENT_ID),
      uuidToBytes(FILE_ID),
      'grant',
      1,
      BigInt(1_700_000_003)
    )
    if (!der) throw new Error('failed to encode')
    const prevBytes = cryptfns.uint8.fromBase64(r1.this_event_hash)
    const buf = new Uint8Array(AUDIT_EVENT_V1_PREFIX.length + prevBytes.length + der.length)
    buf.set(AUDIT_EVENT_V1_PREFIX, 0)
    buf.set(prevBytes, AUDIT_EVENT_V1_PREFIX.length)
    buf.set(der, AUDIT_EVENT_V1_PREFIX.length + prevBytes.length)
    const r3Hash = cryptfns.uint8.toBase64(cryptfns.uint8.fromHex(cryptfns.sha256.digest(buf)))
    const sig = await shareCrypto.signAuditEvent(
      shareCrypto.buildAuditEventSigInput({
        senderId: SENDER_ID,
        recipientId: RECIPIENT_ID,
        fileId: FILE_ID,
        action: 'grant',
        shareRoleBefore: null,
        shareRoleAfter: 'editor',
        timestamp: BigInt(1_700_000_003)
      }),
      kp.input as string
    )
    const r3: ShareEvent = {
      id: 'event-link-broken',
      sender_id: SENDER_ID,
      recipient_id: RECIPIENT_ID,
      file_id: FILE_ID,
      action: 'grant',
      share_role_before: null,
      share_role_after: 'editor',
      created_at: 1_700_000_003,
      prev_event_hash: r1.this_event_hash,
      this_event_hash: r3Hash,
      sender_signature: sig,
      encrypted_name: null,
      cipher: null,
      encrypted_key: null
    }
    const users: Record<string, AuditUserRef> = {
      [SENDER_ID]: {
        id: SENDER_ID,
        email: 'alice@example.com',
        pubkey: kp.publicKey as string,
        fingerprint: 'fp'
      }
    }
    const page: ShareEventPage = {
      events: [r3, r2, r1],
      users,
      total: 3,
      limit: 100,
      offset: 0
    }
    vi.spyOn(sharesApi, 'getShareEvents').mockResolvedValue(page)
    const router = setupRouter()
    router.push('/share/audit')
    await router.isReady()
    const wrapper = mount(ShareHubAudit, { global: { plugins: [router] } })
    await flushPromises()
    expect(
      wrapper.find(`[data-testid="share-hub-audit-row-${r3.id}-tampered-banner"]`).exists()
    ).toBe(true)
  })

  it('audit_verified_and_page_boundary_rows_have_no_tampered_banner', async () => {
    // A page-boundary row (predecessor outside the slice) is not a
    // tamper indicator. The verifier classifies it as `page-boundary`
    // and the row stays silent at the row level.
    const kp = await cryptfns.rsa.generateKeyPair()
    const r1 = await buildRow(0, kp.input as string, null, { created_at: 1_700_000_001 })
    const outOfPageHash = cryptfns.uint8.toBase64(new Uint8Array(32).fill(9))
    const r2 = await buildRow(1, kp.input as string, outOfPageHash, {
      created_at: 1_700_000_002
    })
    const users: Record<string, AuditUserRef> = {
      [SENDER_ID]: {
        id: SENDER_ID,
        email: 'alice@example.com',
        pubkey: kp.publicKey as string,
        fingerprint: 'fp'
      }
    }
    const page: ShareEventPage = {
      events: [r2, r1],
      users,
      total: 2,
      limit: 100,
      offset: 0
    }
    vi.spyOn(sharesApi, 'getShareEvents').mockResolvedValue(page)
    const router = setupRouter()
    router.push('/share/audit')
    await router.isReady()
    const wrapper = mount(ShareHubAudit, { global: { plugins: [router] } })
    await flushPromises()
    expect(
      wrapper.find(`[data-testid="share-hub-audit-row-${r1.id}-tampered-banner"]`).exists()
    ).toBe(false)
    expect(
      wrapper.find(`[data-testid="share-hub-audit-row-${r2.id}-tampered-banner"]`).exists()
    ).toBe(false)
  })

  it('audit_disclosure_on_system_row_omits_legacy_signature_copy', async () => {
    // Cascade-revoke rows legitimately have no sender
    // signature. The disclosure must NOT mention "legacy / pre-Phase 3"
    // (this feature has no legacy) and must NOT name the signature
    // absence as a problem on a system row — the System pill carries
    // that meaning.
    const cascadeFileId = '44444444-4444-4444-4444-444444444444'
    const previousCascadeHash = cryptfns.uint8.toBase64(new Uint8Array(32).fill(7))
    const wasm = await import('../../services/cryptfns/wasm')
    const der = wasm.audit_event_encode_v1(
      new Uint8Array(16),
      uuidToBytes(RECIPIENT_ID),
      uuidToBytes(cascadeFileId),
      'shared_folder_evict',
      0xff,
      BigInt(1_700_000_100)
    )
    if (!der) throw new Error('failed to encode')
    const prev = cryptfns.uint8.fromBase64(previousCascadeHash)
    const buffer = new Uint8Array(AUDIT_EVENT_V1_PREFIX.length + prev.length + der.length)
    buffer.set(AUDIT_EVENT_V1_PREFIX, 0)
    buffer.set(prev, AUDIT_EVENT_V1_PREFIX.length)
    buffer.set(der, AUDIT_EVENT_V1_PREFIX.length + prev.length)
    const cascadeRow: ShareEvent = {
      id: 'cascade-disclosure',
      sender_id: null,
      recipient_id: RECIPIENT_ID,
      file_id: cascadeFileId,
      action: 'shared_folder_evict',
      share_role_before: 'reader',
      share_role_after: null,
      created_at: 1_700_000_100,
      prev_event_hash: previousCascadeHash,
      this_event_hash: cryptfns.uint8.toBase64(
        cryptfns.uint8.fromHex(cryptfns.sha256.digest(buffer))
      ),
      sender_signature: null,
      encrypted_name: null,
      cipher: null,
      encrypted_key: null
    }
    const page: ShareEventPage = {
      events: [cascadeRow],
      users: {},
      total: 1,
      limit: 100,
      offset: 0
    }
    vi.spyOn(sharesApi, 'getShareEvents').mockResolvedValue(page)
    const router = setupRouter()
    router.push('/share/audit')
    await router.isReady()
    const wrapper = mount(ShareHubAudit, { global: { plugins: [router] } })
    await flushPromises()
    await wrapper
      .get(`[data-testid="share-hub-audit-row-${cascadeRow.id}-toggle"]`)
      .trigger('click')
    await flushPromises()
    const disclosure = wrapper.get(
      `[data-testid="share-hub-audit-row-${cascadeRow.id}-disclosure"]`
    )
    expect(disclosure.text()).not.toContain('legacy')
    expect(disclosure.text()).not.toContain('pre-Phase')
    expect(disclosure.text()).not.toContain('Sender signature missing')
  })

  it('audit_disclosure_uses_plain_english_chain_copy_on_page_boundary', async () => {
    // Replace the "Linked to in-page predecessor OR page-
    // boundary (correct under your visibility filter)" jargon with
    // plain English. Verified rows hide the chain row entirely; only
    // page-boundary and tampered states surface a chain copy.
    const kp = await cryptfns.rsa.generateKeyPair()
    const outOfPageHash = cryptfns.uint8.toBase64(new Uint8Array(32).fill(9))
    const row = await buildRow(0, kp.input as string, outOfPageHash, {
      created_at: 1_700_000_002
    })
    const users: Record<string, AuditUserRef> = {
      [SENDER_ID]: {
        id: SENDER_ID,
        email: 'alice@example.com',
        pubkey: kp.publicKey as string,
        fingerprint: 'fp'
      }
    }
    const page: ShareEventPage = {
      events: [row],
      users,
      total: 1,
      limit: 100,
      offset: 0
    }
    vi.spyOn(sharesApi, 'getShareEvents').mockResolvedValue(page)
    const router = setupRouter()
    router.push('/share/audit')
    await router.isReady()
    const wrapper = mount(ShareHubAudit, { global: { plugins: [router] } })
    await flushPromises()
    await wrapper.get(`[data-testid="share-hub-audit-row-${row.id}-toggle"]`).trigger('click')
    await flushPromises()
    const disclosure = wrapper.get(`[data-testid="share-hub-audit-row-${row.id}-disclosure"]`)
    expect(disclosure.text()).toContain('Earlier event in this chain is on another page')
    expect(disclosure.text()).not.toContain('Linked to in-page predecessor')
    expect(disclosure.text()).not.toContain('visibility filter')
  })

  it('audit_row_sentence_describes_grant_with_role_and_recipient', async () => {
    const kp = await cryptfns.rsa.generateKeyPair()
    const row = await buildRow(0, kp.input as string, null, {
      created_at: 1_700_000_001,
      share_role_after: 'editor'
    })
    const users: Record<string, AuditUserRef> = {
      [SENDER_ID]: {
        id: SENDER_ID,
        email: 'alice@example.com',
        pubkey: kp.publicKey as string,
        fingerprint: 'fp'
      },
      [RECIPIENT_ID]: {
        id: RECIPIENT_ID,
        email: 'bob@example.com',
        pubkey: 'pub',
        fingerprint: 'fp'
      }
    }
    const page: ShareEventPage = {
      events: [row],
      users,
      total: 1,
      limit: 100,
      offset: 0
    }
    vi.spyOn(sharesApi, 'getShareEvents').mockResolvedValue(page)
    const router = setupRouter()
    router.push('/share/audit')
    await router.isReady()
    const wrapper = mount(ShareHubAudit, { global: { plugins: [router] } })
    await flushPromises()
    const sentence = wrapper.get(`[data-testid="share-hub-audit-row-${row.id}-sentence"]`)
    expect(sentence.text()).toContain('alice@example.com')
    expect(sentence.text()).toContain('shared')
    expect(sentence.text()).toContain('bob@example.com')
    expect(sentence.text()).toContain('as Editor')
  })

  it('audit_filters_by_action_only_show_matching_events', async () => {
    const kp = await cryptfns.rsa.generateKeyPair()
    const grantRow = await buildRow(0, kp.input as string, null, { created_at: 1_700_000_001 })
    const revokeRow = await buildRow(1, kp.input as string, grantRow.this_event_hash, {
      action: 'revoke',
      created_at: 1_700_000_002,
      share_role_before: 'editor',
      share_role_after: null
    })
    const users: Record<string, AuditUserRef> = {
      [SENDER_ID]: {
        id: SENDER_ID,
        email: 'alice@example.com',
        pubkey: kp.publicKey as string,
        fingerprint: 'fp'
      }
    }
    const spy = vi.spyOn(sharesApi, 'getShareEvents').mockImplementation(async (query) => {
      const filtered =
        query.action === 'revoke'
          ? [revokeRow]
          : query.action === 'grant'
            ? [grantRow]
            : [revokeRow, grantRow]
      return {
        events: filtered,
        users,
        total: filtered.length,
        limit: 100,
        offset: 0
      }
    })
    const router = setupRouter()
    router.push('/share/audit')
    await router.isReady()
    const wrapper = mount(ShareHubAudit, { global: { plugins: [router] } })
    await flushPromises()
    expect(wrapper.findAll('[data-testid="share-hub-audit-list"] > li').length).toBe(2)
    const select = wrapper.get('[data-testid="share-hub-audit-action-filter"]')
    await select.setValue('revoke')
    await flushPromises()
    expect(spy).toHaveBeenLastCalledWith(expect.objectContaining({ action: 'revoke' }))
    // Store now has only revokeRow; refresh redraws the list.
    const store = sharesStore()
    expect(store.events.length).toBe(1)
    expect(store.events[0].action).toBe('revoke')
  })

  it('audit_row_renders_decrypted_file_name_when_join_columns_present', async () => {
    const kp = await cryptfns.rsa.generateKeyPair()
    const encryptedParts = await storageMeta.encrypt(
      { name: 'dogfood-test.png' },
      kp.publicKey as string
    )
    const row = await buildRow(0, kp.input as string, null, {
      action: 'grant',
      share_role_after: 'reader',
      encrypted_name: encryptedParts.encrypted_name,
      cipher: encryptedParts.cipher,
      encrypted_key: encryptedParts.encrypted_key
    })
    const users: Record<string, AuditUserRef> = {
      [SENDER_ID]: {
        id: SENDER_ID,
        email: 'alice@example.com',
        pubkey: kp.publicKey as string,
        fingerprint: 'fp'
      }
    }
    vi.spyOn(sharesApi, 'getShareEvents').mockResolvedValue({
      events: [row],
      users,
      total: 1,
      limit: 100,
      offset: 0
    })
    const router = setupRouter()
    router.push('/share/audit')
    await router.isReady()
    const wrapper = mount(ShareHubAudit, {
      global: { plugins: [router] },
      props: { keypair: kp }
    })
    await flushPromises()
    // Re-flush to give the async decryption batch time to land.
    await flushPromises()
    const sentence = wrapper.get(`[data-testid="share-hub-audit-row-${row.id}-sentence"]`)
    expect(sentence.text()).toContain('dogfood-test.png')
    expect(sentence.text()).not.toContain('file ' + row.file_id.slice(0, 8))
  })

  it('audit_row_falls_back_to_truncated_id_when_encrypted_key_is_null', async () => {
    const kp = await cryptfns.rsa.generateKeyPair()
    const encryptedParts = await storageMeta.encrypt(
      { name: 'hidden.png' },
      kp.publicKey as string
    )
    const row = await buildRow(0, kp.input as string, null, {
      action: 'revoke',
      share_role_before: 'reader',
      share_role_after: null,
      encrypted_name: encryptedParts.encrypted_name,
      cipher: encryptedParts.cipher,
      encrypted_key: null
    })
    const users: Record<string, AuditUserRef> = {
      [SENDER_ID]: {
        id: SENDER_ID,
        email: 'alice@example.com',
        pubkey: kp.publicKey as string,
        fingerprint: 'fp'
      }
    }
    vi.spyOn(sharesApi, 'getShareEvents').mockResolvedValue({
      events: [row],
      users,
      total: 1,
      limit: 100,
      offset: 0
    })
    const router = setupRouter()
    router.push('/share/audit')
    await router.isReady()
    const wrapper = mount(ShareHubAudit, {
      global: { plugins: [router] },
      props: { keypair: kp }
    })
    await flushPromises()
    await flushPromises()
    const sentence = wrapper.get(`[data-testid="share-hub-audit-row-${row.id}-sentence"]`)
    expect(sentence.text()).toContain(`file ${row.file_id.slice(0, 8)}`)
    expect(sentence.text()).not.toContain('hidden.png')
  })

  it('audit_row_falls_back_to_truncated_id_when_encrypted_name_is_null', async () => {
    const kp = await cryptfns.rsa.generateKeyPair()
    const row = await buildRow(0, kp.input as string, null, {
      action: 'grant',
      share_role_after: 'reader',
      encrypted_name: null,
      cipher: null,
      encrypted_key: 'wrap-irrelevant-because-name-is-null'
    })
    const users: Record<string, AuditUserRef> = {
      [SENDER_ID]: {
        id: SENDER_ID,
        email: 'alice@example.com',
        pubkey: kp.publicKey as string,
        fingerprint: 'fp'
      }
    }
    vi.spyOn(sharesApi, 'getShareEvents').mockResolvedValue({
      events: [row],
      users,
      total: 1,
      limit: 100,
      offset: 0
    })
    const router = setupRouter()
    router.push('/share/audit')
    await router.isReady()
    const wrapper = mount(ShareHubAudit, {
      global: { plugins: [router] },
      props: { keypair: kp }
    })
    await flushPromises()
    await flushPromises()
    const sentence = wrapper.get(`[data-testid="share-hub-audit-row-${row.id}-sentence"]`)
    expect(sentence.text()).toContain(`file ${row.file_id.slice(0, 8)}`)
  })

  it('audit_sender_email_filter_resolves_via_discover_then_filters_by_user_id', async () => {
    const kp = await cryptfns.rsa.generateKeyPair()
    const otherSenderId = '99999999-9999-9999-9999-999999999999'
    const aliceRow = await buildRow(0, kp.input as string, null, {
      sender_id: SENDER_ID,
      created_at: 1_700_000_010,
      id: 'event-alice'
    })
    const carolRow = await buildRow(1, kp.input as string, aliceRow.this_event_hash, {
      sender_id: otherSenderId,
      created_at: 1_700_000_011,
      id: 'event-carol'
    })
    const users: Record<string, AuditUserRef> = {
      [SENDER_ID]: {
        id: SENDER_ID,
        email: 'alice@example.com',
        pubkey: kp.publicKey as string,
        fingerprint: 'fp-alice'
      },
      [otherSenderId]: {
        id: otherSenderId,
        email: 'carol@example.com',
        pubkey: kp.publicKey as string,
        fingerprint: 'fp-carol'
      }
    }
    vi.spyOn(sharesApi, 'getShareEvents').mockResolvedValue({
      events: [carolRow, aliceRow],
      users,
      total: 2,
      limit: 100,
      offset: 0
    })
    const discoverSpy = vi.spyOn(sharesApi, 'discoverUser').mockResolvedValue({
      user_id: SENDER_ID,
      email: 'alice@example.com',
      pubkey: kp.publicKey as string,
      fingerprint: 'fp-alice'
    })

    const router = setupRouter()
    router.push('/share/audit')
    await router.isReady()
    const wrapper = mount(ShareHubAudit, {
      global: { plugins: [router] },
      props: { keypair: kp }
    })
    await flushPromises()
    expect(wrapper.findAll('[data-testid="share-hub-audit-list"] > li').length).toBe(2)

    const input = wrapper.get('[data-testid="share-hub-audit-sender-filter"]')
    await input.setValue('alice@example.com')
    await wrapper.get('[data-testid="share-hub-audit-sender-resolve"]').trigger('click')
    await flushPromises()

    expect(discoverSpy).toHaveBeenCalledWith('alice@example.com')
    const rows = wrapper.findAll('[data-testid="share-hub-audit-list"] > li')
    expect(rows.length).toBe(1)
    expect(rows[0].text()).toContain('alice@example.com')
    expect(rows[0].text()).not.toContain('carol@example.com')
  })

  it('audit_sender_email_filter_renders_inline_not_found_error', async () => {
    const kp = await cryptfns.rsa.generateKeyPair()
    const row = await buildRow(0, kp.input as string, null, { id: 'event-1' })
    vi.spyOn(sharesApi, 'getShareEvents').mockResolvedValue({
      events: [row],
      users: {
        [SENDER_ID]: {
          id: SENDER_ID,
          email: 'alice@example.com',
          pubkey: kp.publicKey as string,
          fingerprint: 'fp'
        }
      },
      total: 1,
      limit: 100,
      offset: 0
    })
    vi.spyOn(sharesApi, 'discoverUser').mockRejectedValue(
      new sharesApi.DiscoverUserError('not_found', 'no such user')
    )

    const router = setupRouter()
    router.push('/share/audit')
    await router.isReady()
    const wrapper = mount(ShareHubAudit, {
      global: { plugins: [router] },
      props: { keypair: kp }
    })
    await flushPromises()
    await wrapper.get('[data-testid="share-hub-audit-sender-filter"]').setValue('ghost@nowhere.local')
    await wrapper.get('[data-testid="share-hub-audit-sender-resolve"]').trigger('click')
    await flushPromises()

    const err = wrapper.get('[data-testid="share-hub-audit-sender-error"]')
    expect(err.text().toLowerCase()).toContain("couldn't find")
    // The visible event list is untouched — we don't blank the table on a
    // failed lookup, just attach the inline error to the input.
    expect(wrapper.findAll('[data-testid="share-hub-audit-list"] > li').length).toBe(1)
  })

  it('audit_sender_email_filter_renders_rate_limited_error', async () => {
    const kp = await cryptfns.rsa.generateKeyPair()
    const row = await buildRow(0, kp.input as string, null, { id: 'event-1' })
    vi.spyOn(sharesApi, 'getShareEvents').mockResolvedValue({
      events: [row],
      users: {
        [SENDER_ID]: {
          id: SENDER_ID,
          email: 'alice@example.com',
          pubkey: kp.publicKey as string,
          fingerprint: 'fp'
        }
      },
      total: 1,
      limit: 100,
      offset: 0
    })
    vi.spyOn(sharesApi, 'discoverUser').mockRejectedValue(
      new sharesApi.DiscoverUserError('rate_limited', 'Too many discovery requests.', 30)
    )

    const router = setupRouter()
    router.push('/share/audit')
    await router.isReady()
    const wrapper = mount(ShareHubAudit, {
      global: { plugins: [router] },
      props: { keypair: kp }
    })
    await flushPromises()
    await wrapper.get('[data-testid="share-hub-audit-sender-filter"]').setValue('flood@example.com')
    await wrapper.get('[data-testid="share-hub-audit-sender-resolve"]').trigger('click')
    await flushPromises()

    const err = wrapper.get('[data-testid="share-hub-audit-sender-error"]')
    expect(err.text().toLowerCase()).toContain('too many')
  })
})
