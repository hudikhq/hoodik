import * as meta from '!/storage/meta'

import type {
  AppFile,
  AuditEventActionWire,
  CascadeEntry,
  CreateShareEnvelope,
  DiscoveredUser,
  FolderMember,
  ShareEntryInput,
  ShareRole
} from 'types'

import * as api from './api'
import * as crypto from './crypto'

/**
 * Hard cap on the number of files a single share request may carry.
 * Mirrors the server-side `entries_too_many` limit and the
 * wall-clock budget the server enforces on a share request.
 */
export const SUBTREE_HARD_CAP = 5000

/**
 * Threshold above which the dialog should swap from an indeterminate
 * spinner to a determinate progress bar with running counts.
 */
export const SUBTREE_DETERMINATE_THRESHOLD = 1000

export class SubtreeCapExceeded extends Error {
  readonly count: number
  constructor(count: number) {
    super(
      `This folder has more than ${SUBTREE_HARD_CAP.toLocaleString('en-US')} files. ` +
        `Please share a sub-folder, or split the share.`
    )
    this.count = count
    this.name = 'SubtreeCapExceeded'
  }
}

export class SubtreeAborted extends Error {
  constructor() {
    super('Subtree walk cancelled by the user')
    this.name = 'SubtreeAborted'
  }
}

export interface SubtreeProgress {
  /** Files discovered so far. */
  walked: number
  /** Directories visited so far. */
  directories: number
}

export interface SubtreeWalkOptions {
  /** Abort handle. When `signal.aborted` flips true, the walk throws `SubtreeAborted`. */
  signal?: AbortSignal
  /** Called after each batch of children is fetched so the UI can paint. */
  onProgress?: (progress: SubtreeProgress) => void
  /**
   * Override the directory fetch. Tests inject a fake here; production callers
   * leave it undefined to use the real `/api/storage?dir_id=...` endpoint.
   */
  fetchChildren?: (dirId: string) => Promise<AppFile[]>
}

async function defaultFetchChildren(dirId: string): Promise<AppFile[]> {
  const response = await meta.find({ dir_id: dirId })
  return response.children ?? []
}

/**
 * Walk a file's full subtree breadth-first. For non-directories returns the
 * single file. For directories, descends into every nested folder, fetching
 * children via `meta.find` (the same endpoint the file browser uses).
 *
 * Throws `SubtreeCapExceeded` if the total entry count exceeds
 * `SUBTREE_HARD_CAP`; this is checked before the walk is reported complete
 * so the caller never sees an oversize result.
 */
export async function collectSubtree(
  root: AppFile,
  options: SubtreeWalkOptions = {}
): Promise<AppFile[]> {
  const collected: AppFile[] = [root]
  if (root.mime !== 'dir') {
    options.onProgress?.({ walked: 1, directories: 0 })
    return collected
  }

  const fetchChildren = options.fetchChildren ?? defaultFetchChildren
  const queue: string[] = [root.id]
  let directories = 1

  options.onProgress?.({ walked: collected.length, directories })

  while (queue.length > 0) {
    if (options.signal?.aborted) {
      throw new SubtreeAborted()
    }
    const currentDir = queue.shift() as string
    const children = await fetchChildren(currentDir)
    for (const child of children) {
      collected.push(child)
      if (collected.length > SUBTREE_HARD_CAP) {
        throw new SubtreeCapExceeded(collected.length)
      }
      if (child.mime === 'dir') {
        queue.push(child.id)
        directories += 1
      }
    }
    options.onProgress?.({ walked: collected.length, directories })
  }

  return collected
}

export interface BuildEntriesProgress {
  current: number
  total: number
}

export interface BuildEntriesOptions {
  signal?: AbortSignal
  onProgress?: (progress: BuildEntriesProgress) => void
}

/**
 * For every file in `subtree`, decrypt the caller's own wrap of the file key
 * and re-wrap it under the recipient's public key. Returns the typed
 * `ShareEntryInput[]` ready to drop into the `entries` field of the
 * `POST /api/shares` envelope.
 *
 * The decrypt + wrap pair is two asymmetric operations per file; on a
 * 5000-file folder this is the dominant cost of the whole share submission.
 * The `onProgress` callback fires after every file so the UI can update its
 * determinate progress bar.
 */
export async function buildEntriesForSubtree(
  subtree: AppFile[],
  recipient: crypto.RecipientKey,
  privateKey: string,
  options: BuildEntriesOptions = {}
): Promise<ShareEntryInput[]> {
  const entries: ShareEntryInput[] = []
  options.onProgress?.({ current: 0, total: subtree.length })

  for (let i = 0; i < subtree.length; i++) {
    if (options.signal?.aborted) {
      throw new SubtreeAborted()
    }
    const node = subtree[i]
    const fileKeyHex = await crypto.decryptOwnFileKey(node.encrypted_key, privateKey)
    const wrapped = await crypto.wrapForRecipient(fileKeyHex, recipient)
    entries.push({ file_id: node.id, encrypted_key: wrapped })
    options.onProgress?.({ current: i + 1, total: subtree.length })
  }

  return entries
}

export interface ShareEnvelopeArgs {
  root: AppFile
  entries: ShareEntryInput[]
  target: DiscoveredUser
  shareRole: ShareRole
  senderId: string
  privateKey: string
  /** Audit action the server will reconstruct: `grant` for a fresh share,
   *  `role_change` for an existing recipient's role move, `shared_by_co_owner`
   *  for a Co-owner reshare. Defaults to `grant`. */
  action?: AuditEventActionWire
  /** The recipient's existing role, set only for a `role_change` so the
   *  signed canonical carries the right before-value. */
  shareRoleBefore?: ShareRole | null
}

/**
 * Build the signed `CreateShareEnvelope` for one recipient over a
 * pre-wrapped `entries` list. Computes the entries hash, signs the
 * `ShareRequestPayloadV1`, the per-recipient `AuditEventSigInputV1`, and the
 * `MemberSigPayloadV1`, and — for a folder root — the post-share
 * `FolderMemberListV1` over the roster this share produces. Every signature
 * binds the recipient's id, fingerprint, and resulting role so the server
 * re-encodes and verifies the same canonical from its own state.
 *
 * Shared verbatim by the People-tab single-share and the share-to-group
 * fan-out so the wrap + sign path has exactly one implementation.
 */
export async function buildShareEnvelopeForRecipient(
  args: ShareEnvelopeArgs
): Promise<CreateShareEnvelope> {
  const entriesHash = await crypto.computeEntriesHash(args.entries)
  const timestamp = BigInt(Math.floor(Date.now() / 1000))
  const nonce = crypto.randomNonce()
  const payload = crypto.buildSharePayload({
    senderId: args.senderId,
    recipientId: args.target.user_id,
    recipientPubkeyFingerprintHex: args.target.fingerprint,
    shareRole: args.shareRole,
    rootFileId: args.root.id,
    entriesHash,
    timestamp,
    nonce
  })
  const { payloadDer, signature } = await crypto.signSharePayload(payload, args.privateKey)

  const auditInput = crypto.buildAuditEventSigInput({
    senderId: args.senderId,
    recipientId: args.target.user_id,
    fileId: args.root.id,
    action: args.action ?? 'grant',
    shareRoleBefore: args.shareRoleBefore ?? null,
    shareRoleAfter: args.shareRole,
    timestamp
  })
  const eventSignature = await crypto.signAuditEvent(auditInput, args.privateKey)

  const memberSignedAt = Number(timestamp)
  const memberSignature = await crypto.signMember(
    {
      userId: args.target.user_id,
      pubkeyPem: args.target.pubkey,
      pubkeyFingerprintHex: args.target.fingerprint,
      shareRole: args.shareRole,
      signedAt: timestamp
    },
    args.privateKey
  )

  const envelope: CreateShareEnvelope = {
    payload_der: payloadDer,
    signature,
    entries: args.entries,
    event_signature: eventSignature,
    member_signature: memberSignature,
    member_signed_at: memberSignedAt
  }

  const listSig = await buildPostShareListSignature(
    args.root,
    args.target,
    args.shareRole,
    args.senderId,
    args.privateKey,
    memberSignedAt
  )
  if (listSig) {
    envelope.members_list_signature = listSig
  }

  return envelope
}

/**
 * Sign the `FolderMemberListV1` for the roster a folder will have once this
 * share commits. The owner runs it on an initial share; a Co-owner runs the
 * same path for a reshare, with `signed_by_user_id` pointing to themselves.
 * Returns `null` for non-folder targets — `getFolderMembers` 404s on regular
 * files, so the share path stays single-branch from the caller's side.
 */
async function buildPostShareListSignature(
  root: AppFile,
  target: DiscoveredUser,
  newRole: ShareRole,
  senderId: string,
  privateKey: string,
  signedAt: number
): Promise<{ signature: string; signed_at: number; signed_by_user_id: string } | null> {
  let current: Awaited<ReturnType<typeof api.getFolderMembers>>
  try {
    current = await api.getFolderMembers(root.id)
  } catch {
    return null
  }
  const folderOwnerId = current.folder_owner_id
  const byId = new Map(current.members.map((m) => [m.user_id, m]))

  const next = current.members
    .filter((m) => m.user_id !== target.user_id)
    .map((m) => ({
      userId: m.user_id,
      pubkeyFingerprintHex: m.pubkey_fingerprint,
      shareRole: m.share_role,
      isOwner: m.is_owner,
      signedByUserId: m.signed_by_user_id ?? folderOwnerId
    }))
  const existing = byId.get(target.user_id)
  next.push({
    userId: target.user_id,
    pubkeyFingerprintHex: target.fingerprint,
    shareRole: newRole,
    isOwner: existing?.is_owner ?? false,
    signedByUserId: senderId
  })

  const listInput = crypto.buildFolderMemberListInput({
    folderId: root.id,
    folderOwnerId,
    members: next,
    membersSignedAt: BigInt(signedAt)
  })
  const { signature } = await crypto.signFolderMemberList(listInput, privateKey)
  return { signature, signed_at: signedAt, signed_by_user_id: senderId }
}

/**
 * Folder-cascade analog of `buildEntriesForSubtree`: for every node in
 * `subtree`, decrypt the caller's own wrap once, then re-wrap that node's key
 * once per destination member. The result is the `entries` array the cascade
 * `move-into-shared` body carries — one `CascadeEntry` per node, each with one
 * wrap per member. `callerId` flags the caller's own row on each node with
 * `is_owner_of_file`, matching the single-file path the server inherits
 * σ_member from.
 *
 * Cost is `subtree` RSA decrypts plus `subtree × members` key wraps — the
 * dominant cost of a large move, so `onProgress` fires after every node.
 */
export async function buildCascadeEntries(
  subtree: AppFile[],
  members: FolderMember[],
  callerId: string,
  privateKey: string,
  options: BuildEntriesOptions = {}
): Promise<CascadeEntry[]> {
  const entries: CascadeEntry[] = []
  options.onProgress?.({ current: 0, total: subtree.length })

  for (let i = 0; i < subtree.length; i++) {
    if (options.signal?.aborted) {
      throw new SubtreeAborted()
    }
    const node = subtree[i]
    const fileKeyHex = await crypto.decryptOwnFileKey(node.encrypted_key, privateKey)
    const memberKeys = []
    for (const m of members) {
      memberKeys.push({
        user_id: m.user_id,
        encrypted_key: await crypto.wrapForRecipient(fileKeyHex, m),
        is_owner_of_file: m.user_id === callerId
      })
    }
    entries.push({ file_id: node.id, member_keys: memberKeys })
    options.onProgress?.({ current: i + 1, total: subtree.length })
  }

  return entries
}
