import Api from '../api'
import * as cryptfns from '../cryptfns'

import type {
  AppFile,
  CreateFile,
  EncryptedCreateFile,
  FileResponse,
  Parameters,
  KeyPair,
  EncryptedAppFile,
  SearchQuery,
  AppFileEncryptedPart,
  AppFileUnencryptedPart,
  StorageStatsResponse,
  Rename,
  EncryptedRename,
  DeleteManyFiles,
  MoveManyFiles,
  TransferTokenResponse
} from 'types'

/**
 * Take the unencrypted file or thumbnail and encrypt it with the file key.
 *
 * @param cipher  Cipher identifier (default: `cryptfns.cipher.defaultCipher()`).
 *                Controls which algorithm is used for the file key generation,
 *                name encryption, thumbnail encryption, and chunk encryption.
 */
function isRsaKey(pem: string): boolean {
  // RSA PKCS#1 PEMs carry "RSA" in their header line; the Ed25519 identity and
  // hybrid wrapping PEMs never do. A base64 body can't contain the header's
  // space, so matching "BEGIN RSA" is an unambiguous key-type check.
  return (pem || '').toUpperCase().includes('BEGIN RSA')
}

export async function encrypt(
  unencrypted: AppFileUnencryptedPart,
  publicKey: string,
  cipher = cryptfns.cipher.defaultCipher()
): Promise<AppFileEncryptedPart> {
  const key = unencrypted.key ? unencrypted.key : await cryptfns.cipher.generateKey(cipher)

  const encrypted_name = await cryptfns.cipher.encryptString(cipher, unencrypted.name, key)
  const encrypted_thumbnail = unencrypted.thumbnail
    ? await cryptfns.cipher.encryptString(cipher, unencrypted.thumbnail, key)
    : undefined

  const keyHex = cryptfns.uint8.toHex(key)

  // Curve accounts (post migration) wrap the raw key bytes with the hybrid
  // construction; legacy accounts keep RSA on the hex-encoded key.
  const encrypted_key = isRsaKey(publicKey)
    ? await cryptfns.rsa.encryptMessage(keyHex, publicKey)
    : await cryptfns.wrapping.wrap(key, publicKey)

  return {
    encrypted_key,
    encrypted_name,
    encrypted_thumbnail,
    cipher
  }
}

/**
 * Return the unencrypted file parts.
 *
 * Reads `encrypted.cipher` to determine which cipher was used when the file was created.
 * Falls back to `"ascon128a"` for existing files that predate the cipher field.
 */
/**
 * Unwrap a file key from the form it is stored in on the caller's row.
 *
 * Curve accounts wrap the raw key bytes with the hybrid construction; legacy
 * accounts wrap the hex-encoded key with RSA. Split out of [[decrypt]] because
 * search needs keys on their own, with no name or thumbnail to decrypt.
 */
export async function decryptFileKey(
  encryptedKey: string,
  privateKey: string
): Promise<Uint8Array> {
  if (isRsaKey(privateKey)) {
    return cryptfns.uint8.fromHex(await cryptfns.rsa.decryptMessage(privateKey, encryptedKey))
  }

  return cryptfns.wrapping.unwrap(encryptedKey, privateKey)
}

export async function decrypt(
  encrypted: AppFileEncryptedPart,
  privateKey: string
): Promise<AppFileUnencryptedPart> {
  const cipher = encrypted.cipher

  const key = await decryptFileKey(encrypted.encrypted_key, privateKey)

  const name = await cryptfns.cipher.decryptString(cipher, encrypted.encrypted_name, key)
  const thumbnail = encrypted.encrypted_thumbnail
    ? await cryptfns.cipher.decryptString(cipher, encrypted.encrypted_thumbnail, key)
    : undefined

  return {
    key,
    name,
    thumbnail
  }
}

/**
 * Create a file or directory on the server
 */
export async function create(keypair: KeyPair, unencrypted: CreateFile): Promise<AppFile> {
  if (!keypair.publicKey) {
    throw new Error('Cannot create file without public key')
  }

  if (!keypair.input) {
    throw new Error('Cannot create file without private key')
  }

  const wrapPub = keypair.wrappingPublic || keypair.publicKey
  const cipher = unencrypted.cipher || cryptfns.cipher.defaultCipher()
  // Resolved here rather than inside `encrypt` so the file's search key can be
  // derived from the same bytes the content is encrypted with.
  const key = unencrypted.key || (await cryptfns.cipher.generateKey(cipher))
  const encryptedParts = await encrypt({ ...unencrypted, key }, wrapPub, cipher)

  const rootKey = cryptfns.searchRootKey(keypair)
  const fileKey = cryptfns.searchFileKey(key)
  const indexed = unencrypted.name.toLowerCase()

  const createFile: EncryptedCreateFile = {
    search_tokens_root: cryptfns.searchTags(rootKey, indexed),
    search_tokens_file: cryptfns.searchTags(fileKey, indexed),
    name_hash: cryptfns.searchTag(rootKey, unencrypted.name),
    mime: unencrypted.mime,
    size: unencrypted.size,
    chunks: unencrypted.chunks,
    file_id: unencrypted.file_id,
    file_modified_at: unencrypted.file_modified_at,
    md5: unencrypted.md5,
    sha1: unencrypted.sha1,
    sha256: unencrypted.sha256,
    blake2b: unencrypted.blake2b,
    editable: unencrypted.editable,
    ...encryptedParts
  }

  if (unencrypted.content !== undefined) {
    createFile.content_tokens_root = cryptfns.searchTags(rootKey, unencrypted.content)
    createFile.content_tokens_file = cryptfns.searchTags(fileKey, unencrypted.content)
  }

  const response = await Api.post<EncryptedCreateFile, AppFile>(
    '/api/storage',
    undefined,
    createFile
  )

  if (!response?.body?.id) {
    throw new Error('Failed to create file')
  }

  const file = response.body
  const unencryptedPart = await decrypt(file, keypair.wrappingPrivate || keypair.input)

  return {
    ...file,
    ...unencryptedPart
  }
}

/**
 * Rename a file or a directory
 */
export async function rename(
  keypair: KeyPair,
  file: AppFile,
  unencrypted: Rename
): Promise<AppFile> {
  if (!keypair.publicKey) {
    throw new Error('Cannot rename file without public key')
  }

  if (!keypair.input) {
    throw new Error('Cannot rename file without private key')
  }

  const wrapPub = keypair.wrappingPublic || keypair.publicKey
  const encryptedParts = await encrypt({ key: file.key, name: unencrypted.name }, wrapPub)

  if (!file.key) {
    throw new Error('Cannot rename a file without its key')
  }

  const rootKey = cryptfns.searchRootKey(keypair)
  const name = unencrypted.name

  const rename: EncryptedRename = {
    // An editor renaming someone else's file holds the file key but not the
    // owner's root key, so they refresh only the scope they can produce and
    // the server leaves the other one alone. Name tokens only: the body lives
    // in a different source, and sending it here would wipe it.
    search_tokens_root: file.is_owner ? cryptfns.searchTags(rootKey, name) : undefined,
    search_tokens_file: cryptfns.searchTags(cryptfns.searchFileKey(file.key), name),
    name_hash: cryptfns.searchTag(rootKey, unencrypted.name),
    encrypted_name: encryptedParts.encrypted_name
  }

  const response = await Api.put<EncryptedRename, AppFile>(
    `/api/storage/${file.id}`,
    undefined,
    rename
  )

  if (!response?.body?.id) {
    throw new Error('Failed to create file')
  }

  const renamedFile = response.body
  const unencryptedPart = await decrypt(renamedFile, keypair.wrappingPrivate || keypair.input)

  return {
    ...renamedFile,
    ...unencryptedPart
  }
}

/**
 * Get file or directory metadata
 */
export async function get(keypair: KeyPair, file_id: string): Promise<AppFile> {
  if (!keypair.input) {
    throw new Error('Cannot get file without private key')
  }

  const response = await Api.get<AppFile>(`/api/storage/${file_id}/metadata`, undefined)

  if (!response?.body?.id) {
    throw new Error('Failed to get file or directory')
  }

  const file = response.body
  const unencryptedPart = await decrypt(file, keypair.wrappingPrivate || keypair.input)

  return { ...file, ...unencryptedPart }
}

/**
 *  Lookup directory or file by its name and parent directory
 */
export async function getByName(
  keypair: KeyPair,
  name: string,
  parent_id?: string
): Promise<AppFile> {
  if (!keypair.input) {
    throw new Error('Cannot get file without private key')
  }

  const nameHash = cryptfns.searchTag(cryptfns.searchRootKey(keypair), name)

  if (parent_id !== undefined && typeof parent_id !== 'string') {
    parent_id = undefined
  }

  const response = await Api.get<AppFile>(`/api/storage/${nameHash}/name-hash`, { parent_id })

  if (!response?.body?.id) {
    throw new Error('Failed to get file or directory')
  }

  const file = response.body
  const unencryptedPart = await decrypt(file, keypair.wrappingPrivate || keypair.input)

  return { ...file, ...unencryptedPart }
}

/**
 * Get file or directory metadata.
 *
 * The synthetic `__shared_with_me__` parent is a client-only marker; the
 * recipient-side listing lives behind `/api/shares/mine` instead. Short-
 * circuit here so a stray caller (e.g. the sidebar tree auto-expanding a
 * route) never sends the synthetic id to a UUID-parsing endpoint.
 */
export async function find(parameters: Parameters): Promise<FileResponse> {
  // @ts-ignore
  if (typeof parameters.dir_id !== 'undefined' && typeof parameters.dir_id !== 'string') {
    delete parameters.dir_id
  }

  // @ts-ignore
  if (parameters.dir_id === '__shared_with_me__') {
    return { children: [], parents: [] }
  }

  const response = await Api.get<FileResponse>(`/api/storage`, {
    ...parameters,
    compact: true
  })

  return response.body || { children: [], parents: [] }
}

/**
 * Fetch a single file's encrypted thumbnail. Listings only advertise
 * `has_thumbnail`; the blob itself comes from here, one file at a time.
 */
export async function thumbnail(fileId: string): Promise<string | undefined> {
  const response = await Api.get<{ encrypted_thumbnail?: string }>(
    `/api/storage/${fileId}/thumbnail`
  )

  return response.body?.encrypted_thumbnail ?? undefined
}

/**
 * Get users storage stats
 */
export async function stats(): Promise<StorageStatsResponse> {
  const response = await Api.post<undefined, StorageStatsResponse>(`/api/storage/stats`)

  return response.body || { stats: [], used_space: 0, quota: undefined }
}

/**
/**
 * Indexing sends `"{tag}:{weight}"`; the search route wants bare tags, since
 * the weight that ranks a hit is the stored one, not the query's.
 */
function stripWeight(entry: string): string {
  return entry.split(':')[0]
}

/**
 * Full text search. The query is tokenized and tagged here, so neither the
 * plaintext term nor anything reversible reaches the server.
 *
 * Two scopes go out. Root tags cover everything the caller owns and cost one
 * tag per query word however large the drive is. File tags cover files shared
 * *with* the caller, one per (word, file), because those are keyed on each
 * file's own key — which is exactly what lets a share grant skip the index
 * entirely. Callers never send file tags for files they own, so a file can
 * only match through one scope and the weight ranking stays honest.
 */
export async function search(
  input: string,
  keypair: KeyPair,
  sharedKeys: Uint8Array[] = [],
  options?: { dir_id?: string; editable?: boolean; limit?: number }
): Promise<EncryptedAppFile[]> {
  const term = input.toLowerCase()
  const rootKey = cryptfns.searchRootKey(keypair)

  // The whole trimmed query, tagged as one value alongside its tokens. This
  // is what makes pasting a file's content digest find the file: digests are
  // indexed as keyed tags when hashes land, so an exact match needs no
  // special casing, no shape heuristic, and never a plaintext digest on the
  // wire. Costs one extra tag per scope on every query.
  const exact = input.trim().toLowerCase()

  const body: SearchQuery = {
    root_tags: [
      ...cryptfns.searchTags(rootKey, term).map(stripWeight),
      cryptfns.searchTag(rootKey, exact)
    ],
    // Hashed the way create hashes names — raw and case-preserving — so a
    // pasted filename matches the stored `name_hash` byte for byte.
    name_hash: cryptfns.searchTag(rootKey, input.trim()),
    file_tags: sharedKeys.flatMap((key) => {
      const fileKey = cryptfns.searchFileKey(key)
      return [
        ...cryptfns.searchTags(fileKey, term).map(stripWeight),
        cryptfns.searchTag(fileKey, exact)
      ]
    }),
    dir_id: options?.dir_id,
    editable: options?.editable,
    limit: options?.limit ?? 10,
    skip: 0,
    compact: true
  }

  const response = await Api.post<SearchQuery, EncryptedAppFile[]>(
    `/api/storage/search`,
    undefined,
    body
  )

  return response.body || []
}

export interface KeyedHashesUpdate {
  /** The digest keyed under the file's search key — never the bare digest. */
  sha256: string
  /** Digest tags to append to the index, in `"{tag}:{weight}"` form. */
  search_tokens_root?: string[]
  search_tokens_file?: string[]
}

/**
 * Persist a file's keyed content hash to the server, along with the digest
 * tags that make it findable by pasting the digest into search.
 * Uses an upload transfer token so the request succeeds even after the session expires.
 * Returns the updated AppFile record.
 */
export async function updateHashes(fileId: string, update: KeyedHashesUpdate): Promise<AppFile> {
  const { token } = await requestTransferToken(fileId, 'upload')
  const api = new Api({ ...new Api().toJson(), jwtToken: token, refreshToken: undefined })

  const response = await api.make<KeyedHashesUpdate, AppFile>(
    'put',
    `/api/storage/${fileId}/hashes`,
    undefined,
    update
  )

  if (!response?.body?.id) {
    throw new Error('Failed to update file hashes')
  }

  return response.body
}

/**
 * Toggle the `editable` flag on an existing file.
 * Used to convert a regular file into an editable note (or back).
 */
export async function setEditable(
  keypair: KeyPair,
  fileId: string,
  editable: boolean
): Promise<AppFile> {
  if (!keypair.input) {
    throw new Error('Cannot update file without private key')
  }

  const response = await Api.put<{ editable: boolean }, AppFile>(
    `/api/storage/${fileId}/editable`,
    undefined,
    { editable }
  )

  if (!response?.body?.id) {
    throw new Error('Failed to update editable flag')
  }

  const file = response.body
  const unencryptedPart = await decrypt(file, keypair.wrappingPrivate || keypair.input)

  return { ...file, ...unencryptedPart }
}

/**
 * Get file or directory metadata
 */
export async function remove(fileId: string): Promise<void> {
  await Api.delete(`/api/storage/${fileId}`)
}

/**
 * Remove many files and folders at once
 */
export async function removeAll(body: DeleteManyFiles): Promise<void> {
  await Api.post<DeleteManyFiles, undefined>(`/api/storage/delete-many`, undefined, body)
}

/**
 * Remove many files and folders at once
 */
export async function moveMany(body: MoveManyFiles): Promise<void> {
  await Api.post<MoveManyFiles, undefined>(`/api/storage/move-many`, undefined, body)
}

/**
 * Request a long-lived transfer token scoped to a single file and action.
 * The token is a JWT valid for `long_term_session_duration_days` (default 30 days)
 * and can only be used for the specified action on the specified file.
 */
export async function requestTransferToken(
  fileId: string,
  action: 'upload' | 'download'
): Promise<TransferTokenResponse> {
  const response = await Api.post<{ file_id: string; action: string }, TransferTokenResponse>(
    '/api/auth/transfer-token',
    undefined,
    { file_id: fileId, action }
  )

  if (!response?.body?.token) {
    throw new Error('Failed to request transfer token')
  }

  return response.body
}
