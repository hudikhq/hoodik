import type * as subtree from './subtree'

import type {
  AppFile,
  FolderMember,
  FolderMembersResponse,
  MoveIntoSharedCascadeBody,
  TrustedFingerprintsStore,
  UploadMultiKeyBody,
  UploadMultiKeyResponse
} from 'types'

/**
 * Failures coming out of `verifyFolderMemberList`. Each case maps onto a
 * specific upload-blocking condition the UI should explain — the server
 * either tampered with the list, or the uploader's trust chain doesn't
 * cover the claimed signer.
 */
export class FolderMemberListInvalid extends Error {
  readonly reason:
    | 'list_signature_missing'
    | 'list_signature_invalid'
    | 'list_signature_unauthorized_signer'
    | 'member_signature_invalid'
    | 'fingerprint_mismatch'
    | 'unknown_signer'
    | 'owner_missing'
  readonly userId: string | null

  constructor(
    reason: FolderMemberListInvalid['reason'],
    message: string,
    userId: string | null = null
  ) {
    super(message)
    this.name = 'FolderMemberListInvalid'
    this.reason = reason
    this.userId = userId
  }
}

/**
 * Raised when a cached fingerprint disagrees with the one the server
 * returned for a known member. This is a HARD STOP — no
 * automatic recovery, the user has to re-verify out of band.
 */
export class FolderMemberFingerprintChanged extends Error {
  readonly userId: string
  readonly cachedFingerprint: string
  readonly observedFingerprint: string

  constructor(userId: string, cached: string, observed: string) {
    super(
      `Member ${userId} fingerprint changed from ${cached} to ${observed} since last verified.`
    )
    this.name = 'FolderMemberFingerprintChanged'
    this.userId = userId
    this.cachedFingerprint = cached
    this.observedFingerprint = observed
  }
}

export class UploadIntoSharedFolderAborted extends Error {
  constructor() {
    super('Upload cancelled')
    this.name = 'UploadIntoSharedFolderAborted'
  }
}

/**
 * The uploader is asked to confirm a new member's fingerprint before the
 * upload proceeds. `signedByUserId` is whichever signer (folder owner or
 * a current Co-owner) anointed this member, so the UI can render "added
 * by X" alongside the fingerprint.
 */
export interface UnknownMemberPrompt {
  member: FolderMember
  signedByUserId: string | null
}

export interface UploadIntoSharedFolderProgress {
  /** Cumulative count of RSA wraps completed. */
  wrappedKeys: number
  /** Total wraps the pipeline will do — equal to `members.length`. */
  totalKeys: number
  phase: 'verifying-members' | 'wrapping-keys' | 'signing' | 'submitting' | 'done'
}

/**
 * Encrypted parts the caller has already produced before invoking the
 * pipeline. `fileKeyHex` is the AEGIS/Ascon symmetric key the file is
 * encrypted under — the multi-key pipeline wraps it N times, once per
 * folder member, and never sends the raw value to the server.
 */
export interface SharedFolderFilePayload {
  newFileId: string
  parentFileId: string
  fileKeyHex: string
  nameHash: string
  encryptedName: string
  encryptedThumbnail?: string
  mime: string
  size: number
  chunks: number
  sha256?: string
  cipher: string
  editable?: boolean
  fileModifiedAt?: string
  searchTokensHashed?: string[]
}

export interface UploadIntoSharedFolderArgs {
  callerUserId: string
  /** PEM-encoded RSA private key. Used to sign the audit event. */
  callerPrivateKey: string
  /** PEM-encoded caller public key (echoed only — wraps for the caller come from `fileKeyHex`). */
  callerPublicKey: string
  payload: SharedFolderFilePayload
  trustedFingerprints: TrustedFingerprintsStore
  onUnknownMember?: (prompt: UnknownMemberPrompt) => Promise<boolean>
}

export interface UploadIntoSharedFolderOptions {
  signal?: AbortSignal
  onProgress?: (progress: UploadIntoSharedFolderProgress) => void
  /** Test injection: stub the member-list fetch. */
  fetchMembers?: (folderId: string) => Promise<FolderMembersResponse>
  /** Test injection: stub the upload POST. */
  postUpload?: (body: UploadMultiKeyBody) => Promise<UploadMultiKeyResponse>
}

/** Verified destination roster plus the subtree size, handed to the confirm
 * gate before any key is wrapped: how many descendants travel (excluding the
 * moved root) and the members the subtree will be shared with. */
export interface MoveCascadePreview {
  itemCount: number
  members: FolderMember[]
}

export interface MoveIntoSharedFolderArgs {
  callerUserId: string
  callerPrivateKey: string
  /** The moved folder root. Its full subtree is enumerated and re-wrapped. */
  root: AppFile
  destinationFolderId: string
  trustedFingerprints: TrustedFingerprintsStore
  onUnknownMember?: (prompt: UnknownMemberPrompt) => Promise<boolean>
}

export interface MoveIntoSharedFolderOptions {
  signal?: AbortSignal
  /** Per-node re-wrap progress (and the verify/sign/submit phases). */
  onProgress?: (progress: UploadIntoSharedFolderProgress) => void
  /** Subtree walk progress, distinct from the re-wrap progress. */
  onSubtreeProgress?: (progress: subtree.SubtreeProgress) => void
  /**
   * Invoked once after the destination roster is verified and the subtree is
   * counted, but before any key is wrapped. Returning false aborts the move
   * with nothing sent — mirrors the mobile confirm gate.
   */
  confirm?: (preview: MoveCascadePreview) => Promise<boolean>
  /** Test injection: stub the destination member-list fetch. */
  fetchMembers?: (folderId: string) => Promise<FolderMembersResponse>
  /** Test injection: stub the subtree walk. */
  collectSubtree?: (root: AppFile) => Promise<AppFile[]>
  /** Test injection: stub the cascade POST. */
  postMove?: (body: MoveIntoSharedCascadeBody) => Promise<void>
}
