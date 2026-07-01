import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import {
  createShare,
  discoverUser,
  DiscoverUserError,
  getCapabilities,
  revokeShare
} from '../../services/shares/api'
import type { Capabilities, CreateShareResponse, DiscoveredUser } from '../../types/shares'

interface MockedRequest {
  url: string
  method: string
  headers: Record<string, string>
  body: string | null
}

let lastRequest: MockedRequest | null = null

function mockFetch(status: number, body: object | null, headers: Record<string, string> = {}): void {
  vi.stubGlobal('fetch', async (url: string, init: RequestInit) => {
    lastRequest = {
      url,
      method: (init.method ?? 'get').toLowerCase(),
      headers: { ...(init.headers as Record<string, string>) },
      body: typeof init.body === 'string' ? init.body : null
    }
    // `Response` rejects bodies on no-content statuses (204/205/304).
    const allowsBody = status !== 204 && status !== 205 && status !== 304
    return new Response(allowsBody && body ? JSON.stringify(body) : null, {
      status,
      headers: { 'Content-Type': 'application/json', ...headers }
    })
  })
}

beforeEach(() => {
  lastRequest = null
})

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('shares api service', () => {
  it('discover_user_returns_pubkey_and_fingerprint', async () => {
    const payload: DiscoveredUser = {
      user_id: 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
      email: 'bob@example.com',
      pubkey: 'PUB',
      fingerprint: 'FP'
    }
    mockFetch(200, payload)
    const result = await discoverUser('bob@example.com')
    expect(result.user_id).toEqual(payload.user_id)
    expect(result.pubkey).toEqual('PUB')
    expect(lastRequest?.url).toContain('/api/users/discover')
    expect(lastRequest?.url).toContain(encodeURIComponent('bob@example.com'))
  })

  it('discover_user_404_throws_typed_error', async () => {
    mockFetch(404, { status: 404, message: 'not found', context: 'user_not_found' })
    await expect(discoverUser('ghost@example.com')).rejects.toBeInstanceOf(DiscoverUserError)
    try {
      await discoverUser('ghost@example.com')
    } catch (err) {
      expect((err as DiscoverUserError).kind).toEqual('not_found')
    }
  })

  it('discover_user_429_throws_rate_limit_error', async () => {
    mockFetch(
      429,
      { status: 429, message: 'rate limited', context: 'rate_limited' },
      { 'Retry-After': '42' }
    )
    try {
      await discoverUser('flood@example.com')
      throw new Error('expected throw')
    } catch (err) {
      expect(err).toBeInstanceOf(DiscoverUserError)
      const typed = err as DiscoverUserError
      expect(typed.kind).toEqual('rate_limited')
      expect(typed.retryAfterSeconds).toEqual(42)
    }
  })

  it('discover_user_400_cannot_discover_self_throws_self_error', async () => {
    // The server's `Error::BadRequest` lands its discriminator in
    // `message`; `context` stays null for the non-validation path.
    mockFetch(400, {
      status: 400,
      message: 'cannot_discover_self',
      context: null
    })
    try {
      await discoverUser('me@example.com')
      throw new Error('expected throw')
    } catch (err) {
      expect(err).toBeInstanceOf(DiscoverUserError)
      expect((err as DiscoverUserError).kind).toEqual('self')
    }
  })

  it('discover_user_503_feature_disabled_throws_typed_error', async () => {
    mockFetch(503, {
      status: 503,
      message: 'sharing_disabled',
      context: null
    })
    try {
      await discoverUser('bob@example.com')
      throw new Error('expected throw')
    } catch (err) {
      expect(err).toBeInstanceOf(DiscoverUserError)
      expect((err as DiscoverUserError).kind).toEqual('feature_disabled')
    }
  })

  it('discover_user_unrecognised_400_rethrows_raw_error_response', async () => {
    mockFetch(400, {
      status: 400,
      message: 'email_required',
      context: null
    })
    try {
      await discoverUser('')
      throw new Error('expected throw')
    } catch (err) {
      // Unmapped 400 codes must propagate the original ErrorResponse so the
      // caller can surface the server's description rather than swallowing it.
      expect(err).not.toBeInstanceOf(DiscoverUserError)
    }
  })

  it('create_share_envelope_includes_payload_der_signature_entries_event_signature', async () => {
    const response: CreateShareResponse = { shares: [] }
    mockFetch(201, response)
    await createShare({
      payload_der: 'derPAYLOAD',
      signature: 'sigSHARE',
      entries: [{ file_id: 'aaaa-bbbb', encrypted_key: 'keyB64' }],
      event_signature: 'sigEVENT'
    })
    expect(lastRequest?.url).toContain('/api/shares')
    expect(lastRequest?.method).toEqual('post')
    const sentBody = JSON.parse(lastRequest?.body as string)
    expect(sentBody.payload_der).toEqual('derPAYLOAD')
    expect(sentBody.signature).toEqual('sigSHARE')
    expect(sentBody.entries).toHaveLength(1)
    expect(sentBody.event_signature).toEqual('sigEVENT')
  })

  it('get_capabilities_unauthenticated_works', async () => {
    const payload: Capabilities = {
      sharing: { enabled: true, roles: ['reader', 'editor', 'co-owner'] },
      editable_folders: true,
      share_groups: true,
      audit_log: true,
      fork: true
    }
    mockFetch(200, payload)
    const result = await getCapabilities()
    expect(result.sharing.enabled).toBe(true)
    expect(result.sharing.roles).toContain('co-owner')
  })

  it('revoke_share_body_includes_event_signature_and_timestamp', async () => {
    mockFetch(204, {})
    await revokeShare(
      '11111111-1111-1111-1111-111111111111',
      '22222222-2222-2222-2222-222222222222',
      { event_signature: 'sigREVOKE', timestamp: 1_700_000_000 }
    )
    expect(lastRequest?.method).toEqual('delete')
    expect(lastRequest?.url).toContain(
      '/api/shares/11111111-1111-1111-1111-111111111111/22222222-2222-2222-2222-222222222222'
    )
    const body = JSON.parse(lastRequest?.body as string)
    expect(body.event_signature).toEqual('sigREVOKE')
    expect(body.timestamp).toEqual(1_700_000_000)
  })
})
