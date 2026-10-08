import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { Blob as NodeBlob } from 'node:buffer'
import { TextWriter, Uint8ArrayReader, ZipReader } from '@zip.js/zip.js'
import type { AppFile, KeyPair } from 'types'

const listings = new Map<string, Partial<AppFile>[]>()

vi.mock('!/logger', () => ({ warn: vi.fn() }))

vi.mock('../meta', () => ({
  find: vi.fn(async ({ dir_id }: { dir_id: string }) => ({
    children: listings.get(dir_id) || [],
    parents: []
  })),
  decrypt: vi.fn(async (row: { encrypted_name: string }) => ({
    key: new Uint8Array(32),
    name: row.encrypted_name.replace(/^enc:/, '')
  }))
}))

const downloadChunk = vi.fn(async (file: AppFile, chunk: number) =>
  new TextEncoder().encode(`<${file.id}:${chunk}>`)
)

vi.mock('./sync', () => ({
  downloadChunk: (file: AppFile, chunk: number) => downloadChunk(file, chunk)
}))

import { collectTree, downloadAsZip, pickSaveTarget, safeSegment } from './folder'

// jsdom's Blob stringifies the Node Blob parts zip.js builds archives from,
// yielding "[object Blob]"; Node's Blob behaves like a browser's.
vi.stubGlobal('Blob', NodeBlob)

async function readZip(archive: Blob | Uint8Array): Promise<Record<string, string | null>> {
  const bytes = archive instanceof Uint8Array ? archive : new Uint8Array(await archive.arrayBuffer())
  const reader = new ZipReader(new Uint8ArrayReader(bytes))
  const entries: Record<string, string | null> = {}
  for (const entry of await reader.getEntries()) {
    entries[entry.filename] = entry.directory ? null : await entry.getData!(new TextWriter())
  }
  await reader.close()
  return entries
}

const keypair = { input: 'private', publicKey: 'public' } as unknown as KeyPair

function row(id: string, name: string, extra: Partial<AppFile> = {}): Partial<AppFile> {
  return {
    id,
    encrypted_name: `enc:${name}`,
    mime: 'text/plain',
    size: 10,
    chunks: 1,
    finished_upload_at: 1,
    file_modified_at: 1_700_000_000,
    ...extra
  }
}

function dir(id: string, name: string): Partial<AppFile> {
  return row(id, name, { mime: 'dir', size: undefined, chunks: 0, finished_upload_at: undefined })
}

const root = { id: 'root', name: 'Holiday', mime: 'dir' } as AppFile

describe('safeSegment', () => {
  it('keeps names from escaping their folder', () => {
    expect(safeSegment('a/b')).toBe('a_b')
    expect(safeSegment('..\\evil')).toBe('.._evil')
    expect(safeSegment('..')).toBe('_')
    expect(safeSegment('.')).toBe('_')
    expect(safeSegment('')).toBe('_')
    expect(safeSegment(undefined)).toBe('_')
    expect(safeSegment('ok name.txt')).toBe('ok name.txt')
  })
})

describe('collectTree', () => {
  beforeEach(() => {
    listings.clear()
    vi.clearAllMocks()
  })

  it('walks nested folders with paths rooted at the folder', async () => {
    listings.set('root', [row('a', 'a.txt'), dir('sub', 'Sub'), dir('empty', 'Empty')])
    listings.set('sub', [row('b', 'b.txt', { size: 5 }), dir('deep', 'Deep')])
    listings.set('deep', [row('c', 'c.txt', { size: 7 })])

    const tree = await collectTree(keypair, [root])

    expect(tree.files.map((f) => f.path).sort()).toEqual([
      'Holiday/Sub/Deep/c.txt',
      'Holiday/Sub/b.txt',
      'Holiday/a.txt'
    ])
    expect(tree.emptyDirs.map((d) => d.path)).toEqual(['Holiday/Empty'])
    expect(tree.totalBytes).toBe(22)
  })

  it('leaves out files that have not finished uploading', async () => {
    listings.set('root', [row('a', 'a.txt'), row('b', 'b.txt', { finished_upload_at: undefined })])

    const tree = await collectTree(keypair, [root])

    expect(tree.files.map((f) => f.file.id)).toEqual(['a'])
  })

  it('treats a folder holding only unfinished uploads as empty', async () => {
    listings.set('root', [row('b', 'b.txt', { finished_upload_at: undefined })])

    const tree = await collectTree(keypair, [root])

    expect(tree.files).toEqual([])
    expect(tree.emptyDirs.map((d) => d.path)).toEqual(['Holiday'])
  })

  it('numbers colliding names, ignoring case', async () => {
    listings.set('root', [
      row('a', 'notes.txt'),
      row('b', 'Notes.txt'),
      row('c', 'notes.txt'),
      dir('d', 'notes')
    ])

    const tree = await collectTree(keypair, [root])

    expect(tree.files.map((f) => f.path)).toEqual([
      'Holiday/notes.txt',
      'Holiday/Notes (1).txt',
      'Holiday/notes (2).txt'
    ])
    expect(tree.emptyDirs.map((d) => d.path)).toEqual(['Holiday/notes'])
  })

  it('puts each selected row at the top level, numbering clashes', async () => {
    listings.set('root', [row('a', 'a.txt')])
    listings.set('other', [row('b', 'b.txt')])
    const other = { id: 'other', name: 'holiday', mime: 'dir' } as AppFile
    const loose = { id: 'loose', name: 'notes.txt', mime: 'text/plain', size: 4, chunks: 1, finished_upload_at: 1 } as AppFile
    const uploading = { id: 'up', name: 'partial.bin', mime: 'text/plain', size: 9, chunks: 1 } as AppFile

    const tree = await collectTree(keypair, [root, other, loose, uploading])

    expect(tree.files.map((f) => f.path).sort()).toEqual([
      'Holiday/a.txt',
      'holiday (1)/b.txt',
      'notes.txt'
    ])
    expect(tree.totalBytes).toBe(24)
  })

  it('refuses to walk without a private key', async () => {
    await expect(collectTree({} as KeyPair, [root])).rejects.toThrow()
  })
})

function memoryTarget() {
  const chunks: Uint8Array[] = []
  const state = { closed: false, aborted: false }
  const stream = new WritableStream<Uint8Array>({
    write: (chunk) => {
      chunks.push(chunk)
    },
    close: () => {
      state.closed = true
    },
    abort: () => {
      state.aborted = true
    }
  })
  const bytes = () => {
    const out = new Uint8Array(chunks.reduce((total, chunk) => total + chunk.length, 0))
    let offset = 0
    for (const chunk of chunks) {
      out.set(chunk, offset)
      offset += chunk.length
    }
    return out
  }
  return { stream, state, bytes }
}

describe('downloadAsZip', () => {
  beforeEach(() => {
    listings.clear()
    vi.clearAllMocks()
  })

  it('packs every file chunk by chunk and reports progress up to the total', async () => {
    listings.set('root', [
      row('a', 'a.txt', { size: 5 }),
      row('big', 'big.bin', { size: 21, chunks: 3 }),
      row('z', 'zero.txt', { size: 0, chunks: 0 })
    ])

    const reports: [string, number, number][] = []
    const archive = await downloadAsZip(keypair, [root], {
      onProgress: (stage, bytes, total) => reports.push([stage, bytes, total])
    })

    expect(archive?.blob).toBeInstanceOf(NodeBlob)
    expect(archive?.size).toBe(archive?.blob?.size)
    // The empty file is written without a fetch.
    expect(downloadChunk).toHaveBeenCalledTimes(4)

    expect(await readZip(archive?.blob as Blob)).toEqual({
      'Holiday/a.txt': '<a:0>',
      'Holiday/big.bin': '<big:0><big:1><big:2>',
      'Holiday/zero.txt': ''
    })

    expect(reports[0]).toEqual(['processing', 0, 0])
    expect(reports[reports.length - 1]).toEqual(['processing', 26, 26])
    expect(reports.filter(([stage]) => stage === 'downloading').map(([, bytes]) => bytes)).toEqual(
      [0, 5, 12, 19, 26]
    )
  })

  it('keeps nested paths and empty folders in the archive', async () => {
    listings.set('root', [dir('sub', 'Sub'), dir('empty', 'Empty')])
    listings.set('sub', [row('b', 'b.txt')])

    const archive = await downloadAsZip(keypair, [root])

    expect(await readZip(archive?.blob as Blob)).toEqual({
      'Holiday/Empty/': null,
      'Holiday/Sub/b.txt': '<b:0>'
    })
  })

  it('archives several selected rows together', async () => {
    listings.set('root', [row('a', 'a.txt')])
    listings.set('docs', [])
    const docs = { id: 'docs', name: 'Docs', mime: 'dir' } as AppFile
    const loose = { id: 'loose', name: 'notes.txt', mime: 'text/plain', size: 8, chunks: 1, finished_upload_at: 1 } as AppFile

    const archive = await downloadAsZip(keypair, [root, docs, loose])

    expect(await readZip(archive?.blob as Blob)).toEqual({
      'Docs/': null,
      'Holiday/a.txt': '<a:0>',
      'notes.txt': '<loose:0>'
    })
  })

  it('streams the archive to a target instead of building a blob', async () => {
    listings.set('root', [row('a', 'a.txt'), row('big', 'big.bin', { chunks: 2 })])
    const target = memoryTarget()

    const archive = await downloadAsZip(keypair, [root], { target: target.stream })

    expect(archive?.blob).toBeUndefined()
    expect(target.state.closed).toBe(true)
    expect(archive?.size).toBe(target.bytes().length)
    expect(await readZip(target.bytes())).toEqual({
      'Holiday/a.txt': '<a:0>',
      'Holiday/big.bin': '<big:0><big:1>'
    })
  })

  it('reads a bounded number of chunks ahead of the archive', async () => {
    listings.set(
      'root',
      [...new Array(10)].map((_, i) => row(`f${i}`, `f${i}.txt`))
    )

    let release: () => void = () => {}
    const blocked = new Promise<void>((resolve) => (release = resolve))
    downloadChunk.mockImplementationOnce(async (file: AppFile, chunk: number) => {
      await blocked
      return new TextEncoder().encode(`<${file.id}:${chunk}>`)
    })

    const pending = downloadAsZip(keypair, [root])
    await new Promise((resolve) => setTimeout(resolve, 50))

    // The first chunk is stuck, so only the read-ahead window has started.
    expect(downloadChunk).toHaveBeenCalledTimes(4)

    release()
    const archive = await pending
    expect(downloadChunk).toHaveBeenCalledTimes(10)
    expect(Object.keys(await readZip(archive?.blob as Blob))).toHaveLength(10)
  })

  it('stops without an archive when cancelled', async () => {
    listings.set('root', [row('a', 'a.txt'), row('b', 'b.txt')])
    const target = memoryTarget()

    const archive = await downloadAsZip(keypair, [root], {
      isCancelled: () => true,
      target: target.stream
    })

    expect(archive).toBeUndefined()
    expect(downloadChunk).not.toHaveBeenCalled()
    expect(target.state.aborted).toBe(true)
  })

  it('stops and discards the target when cancelled part way', async () => {
    listings.set('root', [row('a', 'a.txt'), row('big', 'big.bin', { chunks: 3 })])
    const target = memoryTarget()
    let cancelled = false
    downloadChunk.mockImplementationOnce(async () => {
      cancelled = true
      return new TextEncoder().encode('<a:0>')
    })

    const archive = await downloadAsZip(keypair, [root], {
      isCancelled: () => cancelled,
      target: target.stream
    })

    expect(archive).toBeUndefined()
    expect(target.state.aborted).toBe(true)
    expect(target.state.closed).toBe(false)
  })

  it('fails the archive and discards the target when a chunk fails', async () => {
    listings.set('root', [row('a', 'a.txt'), row('b', 'b.txt')])
    downloadChunk.mockRejectedValueOnce(new Error('network down'))
    const target = memoryTarget()

    await expect(downloadAsZip(keypair, [root], { target: target.stream })).rejects.toThrow(
      'network down'
    )
    expect(target.state.aborted).toBe(true)
  })
})

describe('pickSaveTarget', () => {
  afterEach(() => {
    delete window.showSaveFilePicker
  })

  it('reports no picker where the browser has none', async () => {
    expect(await pickSaveTarget('a.zip')).toBeUndefined()
  })

  it('returns the picked file, suggesting the archive name', async () => {
    const handle = {} as FileSystemFileHandle
    window.showSaveFilePicker = vi.fn(async () => handle)

    expect(await pickSaveTarget('Holiday.zip')).toBe(handle)
    expect(window.showSaveFilePicker).toHaveBeenCalledWith(
      expect.objectContaining({ suggestedName: 'Holiday.zip' })
    )
  })

  it('tells a dismissed picker apart from a refused one', async () => {
    window.showSaveFilePicker = vi.fn(async () => {
      throw new DOMException('dismissed', 'AbortError')
    })
    expect(await pickSaveTarget('a.zip')).toBeNull()

    window.showSaveFilePicker = vi.fn(async () => {
      throw new DOMException('no activation', 'SecurityError')
    })
    expect(await pickSaveTarget('a.zip')).toBeUndefined()
  })
})
