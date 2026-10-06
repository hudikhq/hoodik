import { BlobWriter, ZipWriter } from '@zip.js/zip.js'
import * as logger from '!/logger'
import * as meta from '../meta'
import { downloadChunk } from './sync'

import type { AppFile, KeyPair } from 'types'

/**
 * zip.js buffers concurrent entries in memory, so entries are added one at a
 * time; reading ahead across file boundaries is what keeps requests in flight
 * on folders of small files. It also bounds the plaintext held at once.
 */
const CHUNKS_AHEAD = 4

export interface FolderEntry {
  /** Forward slashes, no leading slash. */
  path: string
  file: AppFile
}

export interface FolderTree {
  files: FolderEntry[]
  /** A ZIP only implies folders that contain files; these must be added explicitly. */
  emptyDirs: FolderEntry[]
  totalBytes: number
}

export interface FolderDownloadHooks {
  onProgress?: (stage: 'processing' | 'downloading', bytes: number, totalBytes: number) => void
  isCancelled?: () => boolean
  /** Aborted on failure or cancellation. Without one, the archive is built as a `Blob`. */
  target?: WritableStream<Uint8Array>
}

export interface FolderArchive {
  blob?: Blob
  size: number
}

class FolderDownloadCancelled extends Error {
  constructor() {
    super('Folder download cancelled')
  }
}

/**
 * Names are decrypted from server data, so a slash in one must not let it
 * escape its folder when the archive is extracted.
 */
export function safeSegment(name: string | undefined): string {
  // eslint-disable-next-line no-control-regex
  const cleaned = (name || '').replace(/[/\\\u0000-\u001f]/g, '_').trim()
  if (!cleaned || cleaned === '.' || cleaned === '..') return '_'
  return cleaned
}

/**
 * Case-insensitive, since names differing only in case collide when extracted
 * on such file systems.
 */
function uniqueSegment(name: string, taken: Set<string>): string {
  let candidate = name
  let n = 1

  while (taken.has(candidate.toLowerCase())) {
    const dot = name.lastIndexOf('.')
    candidate =
      dot > 0 ? `${name.slice(0, dot)} (${n})${name.slice(dot)}` : `${name} (${n})`
    n++
  }

  taken.add(candidate.toLowerCase())
  return candidate
}

function isDownloadable(file: AppFile): boolean {
  return file.mime === 'dir' || !!file.finished_upload_at
}

/**
 * Each root sits at the archive's top level under its own name, so a single
 * folder extracts into one directory named after it.
 */
export async function collectTree(
  keypair: KeyPair,
  roots: AppFile[],
  isCancelled?: () => boolean
): Promise<FolderTree> {
  const privateKey = keypair.wrappingPrivate || keypair.input
  if (!privateKey) {
    throw new Error('Cannot download a folder without a private key')
  }

  const tree: FolderTree = { files: [], emptyDirs: [], totalBytes: 0 }
  const pending: FolderEntry[] = []
  const topLevel = new Set<string>()

  for (const root of roots.filter(isDownloadable)) {
    const path = uniqueSegment(safeSegment(root.name), topLevel)

    if (root.mime === 'dir') {
      pending.push({ path, file: root })
    } else {
      tree.files.push({ path, file: root })
      tree.totalBytes += root.size || 0
    }
  }

  while (pending.length) {
    if (isCancelled?.()) break

    const dir = pending.shift() as FolderEntry
    const response = await meta.find({ dir_id: dir.file.id })
    const children = await Promise.all(
      (response.children || []).map(async (row) => ({
        ...row,
        ...(await meta.decrypt(row, privateKey))
      }))
    )

    const taken = new Set<string>()
    let downloadable = 0

    for (const child of children) {
      if (!isDownloadable(child)) continue

      const path = `${dir.path}/${uniqueSegment(safeSegment(child.name), taken)}`
      downloadable++

      if (child.mime === 'dir') {
        pending.push({ path, file: child })
      } else {
        tree.files.push({ path, file: child })
        tree.totalBytes += child.size || 0
      }
    }

    if (!downloadable) {
      tree.emptyDirs.push(dir)
    }
  }

  return tree
}

function modifiedAt(file: AppFile): Date {
  const seconds = file.file_modified_at || file.created_at
  return seconds ? new Date(seconds * 1000) : new Date()
}

function chunkCount(file: AppFile): number {
  return file.size && file.chunks ? file.chunks : 0
}

function chunkFeed(files: AppFile[], ahead: number): () => Promise<Uint8Array> {
  const tasks = files.flatMap((file) =>
    [...new Array(chunkCount(file))].map((_, chunk) => ({ file, chunk }))
  )
  const started: (Promise<Uint8Array> | undefined)[] = []
  let cursor = 0

  return () => {
    while (started.length < Math.min(cursor + ahead, tasks.length)) {
      const { file, chunk } = tasks[started.length]
      const request = downloadChunk(file, chunk)
      // Read-ahead left unconsumed by a failure or cancellation must not
      // surface as an unhandled rejection.
      request.catch(() => {})
      started.push(request)
    }

    const next = started[cursor] as Promise<Uint8Array>
    // Lets the chunk be garbage-collected once written.
    started[cursor] = undefined
    cursor++
    return next
  }
}

function fileStream(
  file: AppFile,
  next: () => Promise<Uint8Array>,
  onChunk: (bytes: number) => void,
  isCancelled?: () => boolean
): ReadableStream<Uint8Array> {
  let remaining = chunkCount(file)

  return new ReadableStream<Uint8Array>({
    async pull(controller) {
      if (isCancelled?.()) {
        controller.error(new FolderDownloadCancelled())
        return
      }

      const data = await next()
      onChunk(data.length)
      controller.enqueue(data)

      if (--remaining === 0) {
        controller.close()
      }
    }
  })
}

/**
 * Resolves to `undefined` when cancelled. Any failed file fails the whole
 * archive: a ZIP silently missing files is worse than one the user is told
 * to retry.
 */
export async function downloadAsZip(
  keypair: KeyPair,
  roots: AppFile[],
  hooks: FolderDownloadHooks = {}
): Promise<FolderArchive | undefined> {
  const { onProgress, isCancelled, target } = hooks

  onProgress?.('processing', 0, 0)

  // Taken before the walk so the target is aborted whatever fails.
  const out = target?.getWriter()

  try {
    const tree = await collectTree(keypair, roots, isCancelled)
    if (isCancelled?.()) throw new FolderDownloadCancelled()

    let written = 0
    const sink = out
      ? new WritableStream<Uint8Array>({
          write: async (chunk) => {
            written += chunk.length
            await out.write(chunk)
          },
          close: () => out.close()
        })
      : new BlobWriter('application/zip')

    // Stored, not deflated: most drive content (media, archives, office
    // documents) is already compressed. Workers only serve zip.js's codecs.
    const zip = new ZipWriter(sink, { level: 0, useWebWorkers: false })

    for (const dir of tree.emptyDirs) {
      await zip.add(`${dir.path}/`, null, { directory: true, lastModDate: modifiedAt(dir.file) })
    }

    let received = 0
    const report = () => onProgress?.('downloading', received, tree.totalBytes)
    report()

    const next = chunkFeed(
      tree.files.map((entry) => entry.file),
      CHUNKS_AHEAD
    )

    for (const { path, file } of tree.files) {
      const content = chunkCount(file)
        ? fileStream(
            file,
            next,
            (bytes) => {
              received += bytes
              report()
            },
            isCancelled
          )
        : null

      await zip.add(path, content, { lastModDate: modifiedAt(file) })
    }

    onProgress?.('processing', received, tree.totalBytes)

    const blob = await zip.close()
    return out ? { size: written } : { blob, size: (blob as Blob).size }
  } catch (err) {
    // Rather than leave a truncated archive under the name the user picked.
    await out?.abort(err).catch(() => {})

    if (err instanceof FolderDownloadCancelled || isCancelled?.()) {
      return undefined
    }
    throw err
  }
}

/**
 * Call before anything is awaited in the click handler: the picker needs the
 * click's user activation.
 *
 * Resolves to `null` when the user dismissed the picker, or `undefined` when
 * none could be shown and the archive should be built in memory instead.
 */
export async function pickSaveTarget(
  name: string
): Promise<FileSystemFileHandle | null | undefined> {
  if (typeof window.showSaveFilePicker !== 'function') return undefined

  try {
    return await window.showSaveFilePicker({
      suggestedName: name,
      types: [{ description: 'ZIP', accept: { 'application/zip': ['.zip'] } }]
    })
  } catch (err) {
    if ((err as DOMException)?.name === 'AbortError') return null

    logger.warn('[download] save picker unavailable, building the archive in memory:', err)
    return undefined
  }
}

export function saveBlob(blob: Blob, name: string): void {
  const url = window.URL.createObjectURL(blob)
  const anchor = document.createElement('a')
  anchor.href = url
  anchor.download = name
  anchor.click()
  // Revoking synchronously can cancel the download in some browsers before
  // it has started reading a large blob.
  setTimeout(() => window.URL.revokeObjectURL(url), 60_000)
}
