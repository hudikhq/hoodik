import Api, { ErrorResponse } from '!/api'
import type {
  AddGroupMemberBody,
  AppShare,
  AppShareGroup,
  Capabilities,
  CreateGroupBody,
  CreateShareEnvelope,
  CreateShareResponse,
  DiscoveredUser,
  EvictFromFolderBody,
  FolderMembersResponse,
  ForkBody,
  ForkResponse,
  GroupMemberWithKey,
  GroupRole,
  GroupsResponse,
  IncomingSharePage,
  MoveIntoSharedBody,
  MoveIntoSharedCascadeBody,
  MoveOutOfSharedBody,
  RevokeShareBody,
  ShareEventPage,
  UploadMultiKeyBody,
  UploadMultiKeyResponse
} from 'types'

/**
 * Discriminated error class thrown by `discoverUser` — pages can branch on
 * `kind` without parsing string codes out of the generic ErrorResponse.
 */
export class DiscoverUserError extends Error {
  readonly kind: 'not_found' | 'self' | 'rate_limited' | 'feature_disabled' | 'other'
  readonly retryAfterSeconds: number | null

  constructor(
    kind: DiscoverUserError['kind'],
    message: string,
    retryAfterSeconds: number | null = null
  ) {
    super(message)
    this.kind = kind
    this.retryAfterSeconds = retryAfterSeconds
  }
}

function parseRetryAfter(headers: Record<string, string>): number | null {
  const raw = headers['retry-after']
  if (!raw) return null
  const seconds = parseInt(raw, 10)
  return Number.isFinite(seconds) && seconds > 0 ? seconds : null
}

/**
 * `GET /api/users/discover?email=...` — look up a recipient by email.
 * Wraps the generic ErrorResponse into a `DiscoverUserError` so callers
 * can render the correct message for each documented failure mode.
 */
export async function discoverUser(email: string): Promise<DiscoveredUser> {
  try {
    const response = await Api.get<DiscoveredUser>(`/api/users/discover`, { email })
    if (!response.body) {
      throw new DiscoverUserError('other', 'Empty response from discover endpoint')
    }
    return response.body
  } catch (err) {
    if (err instanceof ErrorResponse) {
      const code = err.body?.message
      if (err.status === 404) {
        throw new DiscoverUserError('not_found', err.description)
      }
      if (err.status === 400 && code === 'cannot_discover_self') {
        throw new DiscoverUserError('self', err.description)
      }
      if (err.status === 429) {
        throw new DiscoverUserError(
          'rate_limited',
          err.description,
          parseRetryAfter(err.headers)
        )
      }
      if (err.status === 503) {
        throw new DiscoverUserError('feature_disabled', err.description)
      }
    }
    throw err
  }
}

/**
 * `GET /api/auth/key-transitions?user_id=...` (or without for self).
 * Returns the list of historical key transitions for TOFU / chain verification
 * after a crypto migration.
 */
export async function getKeyTransitions(userId?: string): Promise<any[]> {
  const params = userId ? { user_id: userId } : undefined
  const resp = await Api.get<any[]>('/api/auth/key-transitions', params)
  return resp.body || []
}

/**
 * `POST /api/shares` — create or upgrade a share. The caller is responsible
 * for building the signed DER payload and wrapping every entry key for the
 * recipient (see `services/shares/crypto.ts`).
 */
export async function createShare(
  envelope: CreateShareEnvelope
): Promise<CreateShareResponse> {
  const response = await Api.post<CreateShareEnvelope, CreateShareResponse>(
    `/api/shares`,
    undefined,
    envelope
  )
  if (!response.body) {
    throw new Error('Empty response from create-share endpoint')
  }
  return response.body
}

/**
 * `DELETE /api/shares/{file_id}/{user_id}` — revoke a recipient row. The
 * body carries the caller's RSA-PSS signature over the corresponding
 * `AuditEventSigInputV1` for the revoke action.
 */
export async function revokeShare(
  fileId: string,
  userId: string,
  body: RevokeShareBody
): Promise<void> {
  await new Api().withRefresh().make(
    'delete',
    `/api/shares/${fileId}/${userId}`,
    undefined,
    body
  )
}

/**
 * `GET /api/shares/{file_id}` — owner-side recipient list.
 */
export async function getShareRecipients(fileId: string): Promise<AppShare[]> {
  const response = await Api.get<AppShare[]>(`/api/shares/${fileId}`)
  if (!Array.isArray(response.body)) {
    throw new Error('Empty response from share-recipients endpoint')
  }
  return response.body
}

/**
 * `GET /api/shares/mine` — recipient-side paged list of incoming shares.
 */
export async function getSharesMine(
  limit?: number,
  offset?: number
): Promise<IncomingSharePage> {
  const response = await Api.get<IncomingSharePage>(`/api/shares/mine`, {
    limit,
    offset,
    compact: true
  })
  if (!response.body) {
    throw new Error('Empty response from /api/shares/mine')
  }
  return response.body
}

/**
 * `GET /api/shares/mine/by/{user_id}` — recipient-side paged list of
 * shares filtered to a single sender.
 */
export async function getSharesMineBy(
  senderId: string,
  limit?: number,
  offset?: number
): Promise<IncomingSharePage> {
  const response = await Api.get<IncomingSharePage>(`/api/shares/mine/by/${senderId}`, {
    limit,
    offset,
    compact: true
  })
  if (!response.body) {
    throw new Error('Empty response from /api/shares/mine/by/{user_id}')
  }
  return response.body
}

/**
 * `GET /api/shares/events` — audit log filtered to the caller.
 */
export async function getShareEvents(query: {
  file_id?: string
  action?: string
  limit?: number
  offset?: number
}): Promise<ShareEventPage> {
  const response = await Api.get<ShareEventPage>(`/api/shares/events`, {
    file_id: query.file_id,
    action: query.action,
    limit: query.limit,
    offset: query.offset
  })
  if (!response.body) {
    throw new Error('Empty response from /api/shares/events')
  }
  return response.body
}

/**
 * `GET /api/capabilities` — public, unauthenticated capability advertisement.
 * On any failure the caller is expected to fail closed; this wrapper just
 * surfaces the network error.
 */
export async function getCapabilities(): Promise<Capabilities> {
  const response = await Api.get<Capabilities>(`/api/capabilities`)
  if (!response.body) {
    throw new Error('Empty response from /api/capabilities')
  }
  return response.body
}

/**
 * `GET /api/shares/folder/{F}/members` — editable-folder member list with
 * the owner's signature, used to re-wrap keys when sharing or uploading
 * into the folder.
 */
export async function getFolderMembers(folderId: string): Promise<FolderMembersResponse> {
  const response = await Api.get<FolderMembersResponse>(`/api/shares/folder/${folderId}/members`)
  if (!response.body) {
    throw new Error('Empty response from folder-members endpoint')
  }
  return response.body
}

/**
 * Thrown when `POST /api/storage/upload-multikey` (or `move-into-shared`)
 * returns 409 `share_membership_changed`. The body carries the fresh
 * member list the server expects so the caller can re-verify fingerprints,
 * re-wrap, and retry without an extra round-trip.
 */
export class ShareMembershipChangedError extends Error {
  readonly currentMembers: FolderMembersResponse
  constructor(currentMembers: FolderMembersResponse) {
    super('Share membership changed')
    this.name = 'ShareMembershipChangedError'
    this.currentMembers = currentMembers
  }
}

interface MembershipChangedPayload {
  code?: string
  current_members?: FolderMembersResponse
}

function parseMembershipChanged(err: ErrorResponse<unknown>): ShareMembershipChangedError | null {
  if (err.status !== 409) return null
  // The server stuffs JSON into Error::Conflict's payload — parse from
  // either body.message or rawBody depending on which the api client
  // populated for this response.
  const candidate = err.body?.message ?? err.rawBody ?? ''
  if (typeof candidate !== 'string' || candidate.length === 0) return null
  try {
    const parsed = JSON.parse(candidate) as MembershipChangedPayload
    if (parsed.code !== 'share_membership_changed') return null
    if (!parsed.current_members) return null
    return new ShareMembershipChangedError(parsed.current_members)
  } catch {
    return null
  }
}

/**
 * `POST /api/storage/upload-multikey`. Creates a new
 * file row inside a shared folder with one wrapped key per current member.
 * Returns the server-assigned `file_id` (always equal to the caller's
 * supplied `new_file_id` so the audit event signature lines up).
 */
export async function uploadMultiKey(
  body: UploadMultiKeyBody
): Promise<UploadMultiKeyResponse> {
  try {
    const response = await Api.post<UploadMultiKeyBody, UploadMultiKeyResponse>(
      `/api/storage/upload-multikey`,
      undefined,
      body
    )
    if (!response.body) {
      throw new Error('Empty response from upload-multikey endpoint')
    }
    return response.body
  } catch (err) {
    if (err instanceof ErrorResponse) {
      const membershipError = parseMembershipChanged(err)
      if (membershipError) throw membershipError
    }
    throw err
  }
}

/**
 * `POST /api/storage/{file_id}/evict-from-folder`.
 * Folder owner severs a contributor's file from the folder; the file
 * persists in the contributor's drive at root.
 */
export async function evictFromFolder(
  fileId: string,
  body: EvictFromFolderBody
): Promise<void> {
  await Api.post<EvictFromFolderBody, void>(
    `/api/storage/${fileId}/evict-from-folder`,
    undefined,
    body
  )
}

/**
 * `POST /api/storage/move-into-shared`. Re-wraps a
 * file the caller owns for every current member of the destination
 * folder, then re-parents it under that folder. The server matches the
 * supplied `member_keys` against the destination's roster (TOCTOU).
 */
export async function moveIntoShared(body: MoveIntoSharedBody): Promise<void> {
  try {
    await Api.post<MoveIntoSharedBody, void>(
      `/api/storage/move-into-shared`,
      undefined,
      body
    )
  } catch (err) {
    if (err instanceof ErrorResponse) {
      const membershipError = parseMembershipChanged(err)
      if (membershipError) throw membershipError
    }
    throw err
  }
}

/**
 * Folder variant of `moveIntoShared`: same `move-into-shared` endpoint, but
 * the body carries one `CascadeEntry` per node (root + every descendant)
 * instead of a flat `member_keys`. The server recomputes the subtree and
 * rejects with `entries_do_not_match_subtree` if the node set doesn't match.
 * 409 `share_membership_changed` is surfaced identically so the caller can
 * re-verify, re-wrap, and retry once.
 */
export async function moveIntoSharedCascade(
  body: MoveIntoSharedCascadeBody
): Promise<void> {
  try {
    await Api.post<MoveIntoSharedCascadeBody, void>(
      `/api/storage/move-into-shared`,
      undefined,
      body
    )
  } catch (err) {
    if (err instanceof ErrorResponse) {
      const membershipError = parseMembershipChanged(err)
      if (membershipError) throw membershipError
    }
    throw err
  }
}

/**
 * Discriminated rejection from `moveOutOfShared`. The server refuses a
 * move-out either because the caller doesn't own the node (`not_owner`, 403)
 * or because the chosen destination is itself a shared folder
 * (`destination_shared`, 400). Any other failure rethrows the raw error.
 */
export class MoveOutRejectedError extends Error {
  readonly reason: 'not_owner' | 'destination_shared'
  constructor(reason: MoveOutRejectedError['reason'], message: string) {
    super(message)
    this.name = 'MoveOutRejectedError'
    this.reason = reason
  }
}

/**
 * `POST /api/storage/move-out-of-shared` — the file's owner detaches an owned
 * node (and its subtree) from the shared folder it lives in. No wraps travel;
 * the server drops every other member's rows across the subtree. This endpoint
 * never races the roster, so there is no 409 to handle.
 */
export async function moveOutOfShared(body: MoveOutOfSharedBody): Promise<void> {
  try {
    await Api.post<MoveOutOfSharedBody, void>(
      `/api/storage/move-out-of-shared`,
      undefined,
      body
    )
  } catch (err) {
    if (err instanceof ErrorResponse) {
      const code = err.body?.message
      if (err.status === 403 && code === 'cannot_move_not_owner') {
        throw new MoveOutRejectedError('not_owner', err.description)
      }
      if (err.status === 400 && code === 'destination_is_shared') {
        throw new MoveOutRejectedError('destination_shared', err.description)
      }
    }
    throw err
  }
}

/**
 * `POST /api/shares/{file_id}/fork` — save a Co-owner's shared file to the
 * caller's own drive. The client has already decrypted, re-encrypted under
 * a fresh key, and wrapped that key with their own pubkey before calling.
 */
export async function forkFile(fileId: string, body: ForkBody): Promise<ForkResponse> {
  const response = await Api.post<ForkBody, ForkResponse>(
    `/api/shares/${fileId}/fork`,
    undefined,
    body
  )
  if (!response.body) {
    throw new Error('Empty response from fork endpoint')
  }
  return response.body
}

/**
 * `GET /api/shares/groups` — list groups the caller owns plus the groups
 * they're a member of. The server returns the two slices
 * pre-split so the UI renders them in separate columns without a regroup.
 */
export async function listGroups(): Promise<GroupsResponse> {
  const response = await Api.get<GroupsResponse>(`/api/shares/groups`)
  if (!response.body) {
    throw new Error('Empty response from /api/shares/groups')
  }
  return response.body
}

/**
 * `POST /api/shares/groups` — create a new group owned by the caller.
 * Duplicate name returns 409 `share_group_name_conflict`.
 */
export async function createGroup(body: CreateGroupBody): Promise<AppShareGroup> {
  const response = await Api.post<CreateGroupBody, AppShareGroup>(
    `/api/shares/groups`,
    undefined,
    body
  )
  if (!response.body) {
    throw new Error('Empty response from /api/shares/groups')
  }
  return response.body
}

/**
 * `DELETE /api/shares/groups/{id}` — owner-only group deletion. Removing
 * the group does not retroactively revoke shares previously cascaded to
 * its members; that's a separate revoke per file.
 */
export async function deleteGroup(groupId: string): Promise<void> {
  await new Api().withRefresh().make('delete', `/api/shares/groups/${groupId}`)
}

/**
 * `POST /api/shares/groups/{id}/members` — add a member to a group. A group
 * is a saved recipient selection, so this is a plain roster insert; the body
 * carries no file keys. The timestamp + nonce guard the write against replay.
 */
export async function addGroupMember(
  groupId: string,
  body: AddGroupMemberBody
): Promise<void> {
  await Api.post<AddGroupMemberBody, void>(
    `/api/shares/groups/${groupId}/members`,
    undefined,
    body
  )
}

/**
 * `DELETE /api/shares/groups/{id}/members/{user_id}` — remove a member.
 * Requires `can_manage_group`, except a member may always remove
 * themselves (self-leave). A group carries no file associations, so removing
 * a member only changes who a future share-to-group reaches; shares already
 * fanned out to them stay in place (revoke per file to drop those).
 */
export async function removeGroupMember(groupId: string, userId: string): Promise<void> {
  await new Api()
    .withRefresh()
    .make('delete', `/api/shares/groups/${groupId}/members/${userId}`)
}

/**
 * `GET /api/shares/groups/{id}/members` — the group's full recipient set:
 * the owner (carried with `group_role: "owner"`) plus every member, each with
 * the pubkey + fingerprint a share-to-group fan-out wraps for. Visible to
 * anyone in the group, since every member is a candidate recipient when a peer
 * shares to the group.
 */
export async function groupMembers(groupId: string): Promise<GroupMemberWithKey[]> {
  const response = await Api.get<GroupMemberWithKey[]>(
    `/api/shares/groups/${groupId}/members`
  )
  if (!Array.isArray(response.body)) {
    throw new Error('Empty response from /api/shares/groups/{id}/members')
  }
  return response.body
}

/**
 * `PUT /api/shares/groups/{id}/members/{user_id}/role` — set a member's
 * group role. Requires `can_manage_group` plus the escalation guard: a
 * co-owner may set reader/editor but never co-owner. Pure roster metadata —
 * no key moves.
 */
export async function setGroupMemberRole(
  groupId: string,
  userId: string,
  groupRole: GroupRole
): Promise<void> {
  await new Api().withRefresh().make<{ group_role: GroupRole }, void>(
    'put',
    `/api/shares/groups/${groupId}/members/${userId}/role`,
    undefined,
    { group_role: groupRole }
  )
}

/**
 * `PATCH /api/shares/groups/{id}` — rename a group. Requires
 * `can_manage_group`; a duplicate per-owner name returns 409
 * `group_name_taken`.
 */
export async function renameGroup(groupId: string, name: string): Promise<AppShareGroup> {
  const response = await new Api().withRefresh().make<{ name: string }, AppShareGroup>(
    'patch',
    `/api/shares/groups/${groupId}`,
    undefined,
    { name }
  )
  if (!response.body) {
    throw new Error('Empty response from PATCH /api/shares/groups/{id}')
  }
  return response.body
}

/**
 * `PATCH /api/users/me` — partial user update. The body currently carries
 * only the share-notifications opt-out flag; future fields land alongside.
 */
export async function patchMe(payload: {
  share_notifications_enabled?: boolean
}): Promise<{ id: string; share_notifications_enabled: boolean }> {
  const response = await new Api().withRefresh().make<typeof payload, {
    id: string
    share_notifications_enabled: boolean
  }>('patch', `/api/users/me`, undefined, payload)
  if (!response.body) {
    throw new Error('Empty response from /api/users/me')
  }
  return response.body
}
