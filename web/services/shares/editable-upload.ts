import * as cryptfns from '!/cryptfns'

import * as api from './api'
import * as shareCrypto from './crypto'
import {
  ensureNotAborted,
  verifyAndReconcile,
  withMembershipRetry,
  wrapForEveryMember
} from './editable-members'

import type {
  UploadIntoSharedFolderArgs,
  UploadIntoSharedFolderOptions,
  SharedFolderFilePayload
} from './editable-types'

import type { FolderMembersResponse, UploadMultiKeyBody, UploadMultiKeyResponse } from 'types'

/**
 * Fetch the folder's member list, verify it, fan-out RSA wraps for every
 * member, sign the `shared_folder_upload` audit event, and POST the
 * multi-key body. If the server returns `409 share_membership_changed`,
 * the pipeline refreshes the roster (running the new members through the
 * TOFU prompt again) and retries once.
 *
 * Throws:
 *   - `UploadIntoSharedFolderAborted` if `signal` fires or the user
 *     declines a TOFU prompt
 *   - `FolderMemberListInvalid` for any verification failure (server
 *     tampering, missing signer, fingerprint mismatch)
 *   - `FolderMemberFingerprintChanged` for a cached-but-different
 *     fingerprint
 *   - `api.ShareMembershipChangedError` if the second attempt's roster is
 *     also stale (unusual — implies racing membership updates)
 */
export async function uploadIntoSharedFolder(
  args: UploadIntoSharedFolderArgs,
  options: UploadIntoSharedFolderOptions = {}
): Promise<UploadMultiKeyResponse> {
  ensureNotAborted(options.signal)
  const fetchMembers = options.fetchMembers ?? api.getFolderMembers

  const response = await fetchMembers(args.payload.parentFileId)
  options.onProgress?.({
    wrappedKeys: 0,
    totalKeys: response.members.length,
    phase: 'verifying-members'
  })
  await verifyAndReconcile(
    response,
    args.callerUserId,
    args.trustedFingerprints,
    args.onUnknownMember
  )

  const submit = async (
    snapshot: FolderMembersResponse
  ): Promise<UploadMultiKeyResponse> => {
    ensureNotAborted(options.signal)
    const memberKeys = await wrapForEveryMember(
      args.payload.fileKeyHex,
      snapshot.members,
      args.callerUserId,
      options.signal,
      options.onProgress
    )
    ensureNotAborted(options.signal)
    options.onProgress?.({
      wrappedKeys: snapshot.members.length,
      totalKeys: snapshot.members.length,
      phase: 'signing'
    })
    const timestamp = Math.floor(Date.now() / 1000)
    const auditInput = shareCrypto.buildAuditEventSigInput({
      senderId: args.callerUserId,
      recipientId: null,
      fileId: args.payload.newFileId,
      action: 'shared_folder_upload',
      shareRoleBefore: null,
      shareRoleAfter: null,
      timestamp: BigInt(timestamp)
    })
    const eventSignature = await shareCrypto.signAuditEvent(
      auditInput,
      args.callerPrivateKey
    )
    ensureNotAborted(options.signal)

    const body: UploadMultiKeyBody = {
      new_file_id: args.payload.newFileId,
      parent_file_id: args.payload.parentFileId,
      name_hash: args.payload.nameHash,
      encrypted_name: args.payload.encryptedName,
      encrypted_thumbnail: args.payload.encryptedThumbnail,
      mime: args.payload.mime,
      size: args.payload.size,
      chunks: args.payload.chunks,
      sha256: args.payload.sha256,
      cipher: args.payload.cipher,
      editable: args.payload.editable,
      file_modified_at: args.payload.fileModifiedAt,
      search_tokens_hashed: args.payload.searchTokensHashed,
      member_keys: memberKeys,
      members_list_snapshot: {
        members_signed_at: snapshot.members_signed_at,
        members_list_signature: snapshot.members_list_signature
      },
      event_signature: eventSignature,
      timestamp
    }
    options.onProgress?.({
      wrappedKeys: snapshot.members.length,
      totalKeys: snapshot.members.length,
      phase: 'submitting'
    })
    const poster = options.postUpload ?? api.uploadMultiKey
    const result = await poster(body)
    options.onProgress?.({
      wrappedKeys: snapshot.members.length,
      totalKeys: snapshot.members.length,
      phase: 'done'
    })
    return result
  }

  return withMembershipRetry(
    response,
    (snapshot) =>
      verifyAndReconcile(
        snapshot,
        args.callerUserId,
        args.trustedFingerprints,
        args.onUnknownMember
      ),
    submit
  )
}

/**
 * Produce a `SharedFolderFilePayload` from a `File` plus a folder-id by
 * running the same encrypt pipeline the regular upload uses (cipher key,
 * encrypted name, encrypted thumbnail). The returned `fileKeyBytes` is
 * the raw symmetric key — needed by the chunk-upload pipeline so chunks
 * are encrypted under the same key wrapped for the folder members.
 */
export async function buildSharedFolderPayloadFromFile(args: {
  newFileId: string
  parentFileId: string
  file: File
  searchTokensHashed: string[]
  cipher?: string
  fileKey?: Uint8Array
  thumbnail?: string
  editable?: boolean
  chunkSizeBytes: number
}): Promise<SharedFolderFilePayload & { fileKeyBytes: Uint8Array }> {
  const cipher = args.cipher ?? cryptfns.cipher.DEFAULT_CIPHER
  const key = args.fileKey ?? (await cryptfns.cipher.generateKey(cipher))
  const encryptedName = await cryptfns.cipher.encryptString(cipher, args.file.name, key)
  const encryptedThumbnail = args.thumbnail
    ? await cryptfns.cipher.encryptString(cipher, args.thumbnail, key)
    : undefined
  const nameHash = cryptfns.sha256.digest(args.file.name)
  const mime = args.file.type || 'application/octet-stream'
  const fileKeyHex = cryptfns.uint8.toHex(key)
  const chunks = Math.max(1, Math.ceil(args.file.size / args.chunkSizeBytes))
  return {
    newFileId: args.newFileId,
    parentFileId: args.parentFileId,
    fileKeyHex,
    fileKeyBytes: key,
    nameHash,
    encryptedName,
    encryptedThumbnail,
    mime,
    size: args.file.size,
    chunks,
    cipher,
    editable: args.editable,
    searchTokensHashed: args.searchTokensHashed
  }
}
