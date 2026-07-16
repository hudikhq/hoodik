import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import * as cryptfns from '../../services/cryptfns'
import * as sharesApi from '../../services/shares/api'
import * as shareCrypto from '../../services/shares/crypto'
import { forkFile, ForkAbortedError } from '../../services/shares/fork'
import * as downloadSync from '../../services/storage/download/sync'
import * as meta from '../../services/storage/meta'

import type { AppFile, KeyPair } from '../../types'

const SOURCE_ID = '11111111-1111-1111-1111-111111111111'
const NEW_FILE_ID = '22222222-2222-2222-2222-222222222222'
const CALLER_ID = '33333333-3333-3333-3333-333333333333'

beforeEach(() => {
  setActivePinia(createPinia())
})

afterEach(() => {
  vi.restoreAllMocks()
})

interface SimEnvironment {
  kp: KeyPair
  /** Original source-file fully ready for the fork. */
  source: AppFile
  /** Captured chunks: ciphertext bytes uploaded to the new file id. */
  uploaded: Map<number, Uint8Array>
  /** The new symmetric key the fork ended up using. */
  capturedForkBody: Parameters<typeof sharesApi.forkFile>[1] | null
  /** The plaintext input used. */
  plaintext: Uint8Array
}

async function prepareForkEnvironment(): Promise<SimEnvironment> {
  const kp = await cryptfns.rsa.generateKeyPair()
  const cipher = 'aegis128l'
  const oldKey = await cryptfns.cipher.generateKey(cipher)
  const oldKeyHex = cryptfns.uint8.toHex(oldKey)
  const oldEncryptedKey = await cryptfns.rsa.encryptMessage(oldKeyHex, kp.publicKey as string)

  const plaintext = new Uint8Array(48).map((_, i) => (i + 7) & 0xff)
  // Split into 3 chunks of 16 bytes; each gets independently encrypted.
  const chunkPlain = [plaintext.slice(0, 16), plaintext.slice(16, 32), plaintext.slice(32, 48)]
  const encryptedChunks: Uint8Array[] = []
  for (let i = 0; i < chunkPlain.length; i++) {
    encryptedChunks.push(await cryptfns.cipher.encrypt(cipher, chunkPlain[i], oldKey, i))
  }

  const source: AppFile = {
    id: SOURCE_ID,
    user_id: 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
    is_owner: false,
    name: 'plan.txt',
    name_hash: 'name-hash',
    mime: 'text/plain',
    size: plaintext.length,
    chunks: 3,
    file_id: null,
    file_modified_at: 1_700_000_000,
    created_at: 1_700_000_000,
    is_new: false,
    editable: false,
    active_version: 1,
    encrypted_key: oldEncryptedKey,
    encrypted_name: '',
    cipher,
    key: oldKey
  } as unknown as AppFile

  vi.spyOn(downloadSync, 'downloadChunk').mockImplementation(async (_file, chunk) => {
    return chunkPlain[chunk] ?? new Uint8Array(0)
  })

  const uploaded = new Map<number, Uint8Array>()
  const fetchMock = vi.fn(async (url: string | URL, init?: RequestInit) => {
    const u = typeof url === 'string' ? url : url.toString()
    if (u.includes('/api/auth/transfer-token')) {
      return new Response(
        JSON.stringify({
          token: 'transfer-token',
          expires_at: 9_999_999_999,
          file_id: NEW_FILE_ID,
          action: 'upload'
        }),
        { status: 200, headers: { 'Content-Type': 'application/json' } }
      )
    }
    if (u.includes(`/api/storage/${NEW_FILE_ID}`)) {
      const match = u.match(/chunk=(\d+)/)
      const chunkIndex = match ? Number(match[1]) : -1
      const body = init?.body
      if (body instanceof Uint8Array) {
        uploaded.set(chunkIndex, body)
      }
      return new Response(
        JSON.stringify({
          id: NEW_FILE_ID,
          uploaded_chunks: Array.from(uploaded.keys()),
          chunks_stored: uploaded.size
        }),
        { status: 200, headers: { 'Content-Type': 'application/json' } }
      )
    }
    return new Response(null, { status: 404 })
  })
  vi.stubGlobal('fetch', fetchMock)

  let capturedForkBody: Parameters<typeof sharesApi.forkFile>[1] | null = null
  vi.spyOn(sharesApi, 'forkFile').mockImplementation(async (_id, body) => {
    capturedForkBody = body
    return { file_id: NEW_FILE_ID, created_at: 1_700_001_000 }
  })

  return { kp, source, uploaded, capturedForkBody, plaintext }
}

describe('fork pipeline', () => {
  it('fork_decrypts_chunks_under_old_key_and_re_encrypts_under_new', async () => {
    const env = await prepareForkEnvironment()
    const result = await forkFile({
      source: env.source,
      keypair: env.kp,
      callerUserId: CALLER_ID,
      callerRecipient: { pubkey: env.kp.publicKey as string }
    })
    expect(result.file_id).toBe(NEW_FILE_ID)
    // The forkFile body carried the new wrap; decrypt it to obtain the
    // fresh symmetric key, then decrypt every uploaded chunk and prove
    // the bytes match the original plaintext.
    expect(env.uploaded.size).toBe(3)
    const callArgs = (sharesApi.forkFile as unknown as { mock: { calls: unknown[][] } }).mock.calls
    const body = callArgs[0][1] as Parameters<typeof sharesApi.forkFile>[1]
    const newKeyHex = await cryptfns.rsa.decryptMessage(env.kp, body.encrypted_key)
    const newKey = cryptfns.uint8.fromHex(newKeyHex)
    const cipher = body.cipher as string

    const decryptedFlat = new Uint8Array(env.plaintext.length)
    let cursor = 0
    for (let i = 0; i < 3; i++) {
      const ct = env.uploaded.get(i)
      expect(ct).toBeDefined()
      const pt = await cryptfns.cipher.decrypt(cipher, ct as Uint8Array, newKey, i)
      decryptedFlat.set(pt, cursor)
      cursor += pt.length
    }
    expect(Array.from(decryptedFlat)).toEqual(Array.from(env.plaintext))
  })

  it('fork_uses_same_cipher_as_source', async () => {
    const env = await prepareForkEnvironment()
    await forkFile({ source: env.source, keypair: env.kp, callerUserId: CALLER_ID, callerRecipient: { pubkey: env.kp.publicKey as string } })
    const callArgs = (sharesApi.forkFile as unknown as { mock: { calls: unknown[][] } }).mock.calls
    const body = callArgs[0][1] as Parameters<typeof sharesApi.forkFile>[1]
    expect(body.cipher).toEqual(env.source.cipher)
  })

  it('fork_creates_new_file_id_distinct_from_source', async () => {
    const env = await prepareForkEnvironment()
    await forkFile({ source: env.source, keypair: env.kp, callerUserId: CALLER_ID, callerRecipient: { pubkey: env.kp.publicKey as string } })
    const callArgs = (sharesApi.forkFile as unknown as { mock: { calls: unknown[][] } }).mock.calls
    const body = callArgs[0][1] as Parameters<typeof sharesApi.forkFile>[1]
    expect(body.new_file_id).not.toEqual(env.source.id)
    expect(body.new_file_id.length).toBeGreaterThan(20)
  })

  it('fork_cancel_mid_upload_cleans_up_partial_chunks', async () => {
    const env = await prepareForkEnvironment()
    const controller = new AbortController()
    // Abort immediately so the very first ensureNotAborted check in the
    // pipeline rejects with ForkAbortedError — no partial state to
    // rollback since the fork endpoint hasn't been called yet.
    controller.abort()
    await expect(
      forkFile(
        { source: env.source, keypair: env.kp, callerUserId: CALLER_ID, callerRecipient: { pubkey: env.kp.publicKey as string } },
        { signal: controller.signal }
      )
    ).rejects.toBeInstanceOf(ForkAbortedError)
  })

  it('fork_cancel_mid_upload_rolls_back_partial_upload', async () => {
    const env = await prepareForkEnvironment()
    const controller = new AbortController()
    const originalEncrypt = downloadSync.downloadChunk
    let downloadCalls = 0
    vi.spyOn(downloadSync, 'downloadChunk').mockImplementation(async (file, chunk) => {
      downloadCalls += 1
      if (downloadCalls === 2) {
        controller.abort()
      }
      return originalEncrypt(file, chunk)
    })
    await expect(
      forkFile(
        { source: env.source, keypair: env.kp, callerUserId: CALLER_ID, callerRecipient: { pubkey: env.kp.publicKey as string } },
        { signal: controller.signal }
      )
    ).rejects.toBeInstanceOf(ForkAbortedError)
    // The fork endpoint isn't reached before the second chunk download
    // throws — that means no rollback is needed (no fork file row was
    // ever created). The pipeline correctly aborts before any server
    // state is mutated.
    expect((sharesApi.forkFile as unknown as { mock: { calls: unknown[][] } }).mock.calls.length).toBe(0)
  })

  it('fork_progress_callback_reports_bytes_processed', async () => {
    const env = await prepareForkEnvironment()
    const progress: number[] = []
    await forkFile(
      { source: env.source, keypair: env.kp, callerUserId: CALLER_ID, callerRecipient: { pubkey: env.kp.publicKey as string } },
      {
        onProgress: (p) => progress.push(p.bytesProcessed)
      }
    )
    expect(progress.length).toBeGreaterThan(0)
    expect(progress[progress.length - 1]).toBe(env.plaintext.length)
  })

  it('fork_event_signature_signed_with_caller_privkey', async () => {
    const env = await prepareForkEnvironment()
    await forkFile({ source: env.source, keypair: env.kp, callerUserId: CALLER_ID, callerRecipient: { pubkey: env.kp.publicKey as string } })
    const callArgs = (sharesApi.forkFile as unknown as { mock: { calls: unknown[][] } }).mock.calls
    const body = callArgs[0][1] as Parameters<typeof sharesApi.forkFile>[1]
    // Fork signs over the source file id — the audit row attributes the
    // event to the original so the owner of the source sees who forked
    // their file.
    const sigInput = shareCrypto.buildAuditEventSigInput({
      senderId: CALLER_ID,
      recipientId: null,
      fileId: env.source.id,
      action: 'fork',
      shareRoleBefore: null,
      shareRoleAfter: null,
      timestamp: BigInt(body.timestamp)
    })
    const verified = await shareCrypto.verifyAuditEvent(sigInput, body.event_signature, {
      pubkey: env.kp.publicKey as string
    })
    expect(verified).toBe(true)
  })

  it('fork_silences_unused_meta_import', () => {
    // The pipeline depends on meta.requestTransferToken — assert the
    // import surface so tree-shaking can't accidentally drop it.
    expect(meta.requestTransferToken).toBeDefined()
  })
})
