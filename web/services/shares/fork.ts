import * as cryptfns from '!/cryptfns'
import Api from '!/api'
import * as meta from '!/storage/meta'
import * as downloadSync from '!/storage/download/sync'
import * as uploadSync from '!/storage/upload/sync'
import * as logger from '!/logger'
import { CHUNK_SIZE_BYTES } from '!/constants'
import { uuidv4 } from '!/index'

import * as api from './api'
import * as shareCrypto from './crypto'
import type { RecipientKey } from './crypto'

import type {
  AppFile,
  ForkBody,
  ForkResponse,
  KeyPair,
  UploadAppFile
} from 'types'

export interface ForkProgress {
  /** Bytes of decrypted plaintext processed so far. */
  bytesProcessed: number
  /** Total plaintext size, when known. Equal to `source.size`. */
  totalBytes: number
  /** Current phase — used by the UI to render a sensible label. */
  phase: 'preparing' | 'downloading' | 'uploading' | 'finalizing' | 'done'
}

export interface ForkOptions {
  signal?: AbortSignal
  onProgress?: (progress: ForkProgress) => void
}

export class ForkAbortedError extends Error {
  constructor() {
    super('Fork was cancelled')
    this.name = 'ForkAbortedError'
  }
}

function ensureNotAborted(signal?: AbortSignal): void {
  if (signal?.aborted) {
    throw new ForkAbortedError()
  }
}

async function rollbackPartialUpload(fileId: string): Promise<void> {
  try {
    await new Api().withRefresh().make('delete', `/api/storage/${fileId}`)
  } catch (err) {
    logger.warn(`[fork] cleanup of partial fork ${fileId} failed:`, err)
  }
}

async function downloadDecryptedChunks(
  source: AppFile,
  options: ForkOptions
): Promise<Uint8Array[]> {
  if (!source.key) {
    throw new Error('Source file is missing its decrypted key')
  }
  const chunks: Uint8Array[] = []
  let processed = 0
  for (let i = 0; i < source.chunks; i++) {
    ensureNotAborted(options.signal)
    const plaintext = await downloadSync.downloadChunk(source, i, options.signal)
    chunks.push(plaintext)
    processed += plaintext.length
    options.onProgress?.({
      bytesProcessed: processed,
      totalBytes: source.size ?? processed,
      phase: 'downloading'
    })
  }
  return chunks
}

function totalPlaintextSize(chunks: Uint8Array[]): number {
  return chunks.reduce((total, chunk) => total + chunk.length, 0)
}

function chunkCount(plaintextChunks: Uint8Array[]): number {
  const totalBytes = totalPlaintextSize(plaintextChunks)
  if (totalBytes === 0) return 0
  return Math.max(plaintextChunks.length, Math.ceil(totalBytes / CHUNK_SIZE_BYTES))
}

async function streamingSha256(chunks: Uint8Array[]): Promise<string> {
  const totalBytes = totalPlaintextSize(chunks)
  const flat = new Uint8Array(totalBytes)
  let cursor = 0
  for (const chunk of chunks) {
    flat.set(chunk, cursor)
    cursor += chunk.length
  }
  return cryptfns.sha256.digest(flat)
}

export interface ForkArgs {
  /** Source file (must have a populated `key`). */
  source: AppFile
  /** Caller's keypair — its private key decrypts the source wrap and signs the audit event. */
  keypair: KeyPair
  /** Caller's user id — folded into the audit-event signature. */
  callerUserId: string
  /**
   * Caller's own key material, used to wrap the fresh key back to
   * themselves. Curve25519 accounts seal under their `wrapping_pubkey`;
   * RSA accounts encrypt under their `pubkey`.
   */
  callerRecipient: RecipientKey
}

/**
 * Fork a shared file into the caller's own drive:
 *
 * 1. download every encrypted chunk under the caller's existing wrap;
 * 2. decrypt each chunk client-side;
 * 3. generate a fresh symmetric key in the same cipher the source uses;
 * 4. re-encrypt every chunk under the new key;
 * 5. wrap the new key for the caller's own pubkey;
 * 6. POST `/api/shares/{source_id}/fork` with the fresh metadata + key;
 * 7. upload every re-encrypted chunk to the new file id via the
 *    standard `POST /api/storage/{file_id}` route;
 * 8. report progress + honour cancellation, rolling back partial
 *    uploads if cancelled mid-way.
 */
export async function forkFile(
  args: ForkArgs,
  options: ForkOptions = {}
): Promise<ForkResponse> {
  const { source, keypair, callerUserId, callerRecipient } = args
  if (!keypair.input) {
    throw new Error('Cannot fork without the caller\'s private key')
  }
  if (!keypair.publicKey) {
    throw new Error('Cannot fork without the caller\'s public key')
  }
  if (!source.encrypted_key) {
    throw new Error('Source file has no wrapped key for the caller')
  }
  if (source.mime === 'dir') {
    throw new Error('Folders cannot be forked')
  }
  ensureNotAborted(options.signal)

  options.onProgress?.({ bytesProcessed: 0, totalBytes: source.size ?? 0, phase: 'preparing' })

  // Decrypt the source's per-user key. `source.key` may already be in
  // place from the page that opened the fork (My-Files row); the
  // explicit decrypt keeps the helper standalone.
  const sourceKeyHex = await shareCrypto.decryptOwnFileKey(
    source.encrypted_key,
    keypair.wrappingPrivate || keypair.input
  )
  const sourceKey = cryptfns.uint8.fromHex(sourceKeyHex)
  const sourceWithKey: AppFile = { ...source, key: sourceKey }

  // 1 + 2 — chunked download + decrypt under the source key.
  const plaintextChunks = await downloadDecryptedChunks(sourceWithKey, options)
  ensureNotAborted(options.signal)

  // 3 — fresh key in the same cipher (preserve files.cipher).
  const cipher = source.cipher
  const newKey = await cryptfns.cipher.generateKey(cipher)
  const newKeyHex = cryptfns.uint8.toHex(newKey)

  // 4 — wrap the new key for the caller under their own key type. The
  // same wrap is stored both in the file row's `encrypted_key` (legacy)
  // and on the per-user row.
  const wrappedKey = await shareCrypto.wrapForRecipient(newKeyHex, callerRecipient)

  // Re-encrypt the source's plaintext name + thumbnail under the new
  // key so the server's encrypted_metadata column carries the right
  // ciphertext for the forked file. Listings no longer carry thumbnail
  // blobs, so pull it from the thumbnail route when the row only
  // advertises one — otherwise the fork would silently lose it.
  let sourceThumbnail = source.thumbnail
  if (!sourceThumbnail && source.has_thumbnail) {
    const encrypted = await meta.thumbnail(source.id)
    if (encrypted) {
      sourceThumbnail = await cryptfns.cipher.decryptString(cipher, encrypted, sourceKey)
    }
  }

  const encryptedName = await cryptfns.cipher.encryptString(cipher, source.name, newKey)
  const encryptedThumbnail = sourceThumbnail
    ? await cryptfns.cipher.encryptString(cipher, sourceThumbnail, newKey)
    : undefined

  const newFileId = uuidv4()
  const timestamp = Math.floor(Date.now() / 1000)
  const sha256 = await streamingSha256(plaintextChunks)
  const totalBytes = source.size ?? totalPlaintextSize(plaintextChunks)
  const expectedChunks = chunkCount(plaintextChunks)

  // Sign over the source file id so the audit row attributes the fork
  // back to the original — the owner of the source
  // sees who saved a copy. The server records and verifies the same id.
  const auditInput = shareCrypto.buildAuditEventSigInput({
    senderId: callerUserId,
    recipientId: null,
    fileId: source.id,
    action: 'fork',
    shareRoleBefore: null,
    shareRoleAfter: null,
    timestamp: BigInt(timestamp)
  })
  const eventSignature = await shareCrypto.signAuditEvent(auditInput, keypair.input)

  const body: ForkBody = {
    new_file_id: newFileId,
    encrypted_metadata: encryptedName,
    encrypted_thumbnail: encryptedThumbnail,
    name_hash: cryptfns.sha256.digest(source.name),
    mime: source.mime,
    size: totalBytes,
    chunks: expectedChunks,
    sha256,
    cipher,
    encrypted_key: wrappedKey,
    search_tokens_hashed: cryptfns.stringToHashedTokens(source.name.toLowerCase()),
    event_signature: eventSignature,
    timestamp
  }

  // 6 — server creates the file + user_files rows + audit row.
  const fork = await api.forkFile(source.id, body)
  ensureNotAborted(options.signal)
  options.onProgress?.({
    bytesProcessed: totalBytes,
    totalBytes,
    phase: 'uploading'
  })

  // 7 — upload re-encrypted chunks via the standard chunk endpoint.
  const placeholder: UploadAppFile = {
    id: fork.file_id,
    user_id: callerUserId,
    is_owner: true,
    name_hash: body.name_hash,
    mime: source.mime,
    chunks: expectedChunks,
    file_modified_at: timestamp,
    created_at: timestamp,
    is_new: true,
    editable: source.editable,
    active_version: 1,
    encrypted_key: wrappedKey,
    encrypted_name: encryptedName,
    encrypted_thumbnail: encryptedThumbnail,
    cipher,
    key: newKey,
    name: source.name,
    size: totalBytes,
    sha256,
    file: new File([new Blob([])], source.name, { type: source.mime })
  }

  const { token } = await meta.requestTransferToken(fork.file_id, 'upload')
  const uploadApi = new Api({ ...new Api().toJson(), jwtToken: token, refreshToken: undefined })

  try {
    let processed = 0
    for (let i = 0; i < plaintextChunks.length; i++) {
      ensureNotAborted(options.signal)
      // `uploadChunk` re-encrypts internally using `file.cipher` +
      // `file.key`; supply the already-decrypted plaintext chunk so
      // the bytes that land on disk match the new key end-to-end.
      await uploadSync.uploadChunk(placeholder, plaintextChunks[i], i, 0, uploadApi)
      processed += plaintextChunks[i].length
      options.onProgress?.({
        bytesProcessed: processed,
        totalBytes,
        phase: 'uploading'
      })
    }
  } catch (err) {
    await rollbackPartialUpload(fork.file_id)
    throw err
  }

  options.onProgress?.({ bytesProcessed: totalBytes, totalBytes, phase: 'done' })
  return fork
}
