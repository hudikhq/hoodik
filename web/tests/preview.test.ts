import { describe, it, expect } from 'vitest'
import { isPreviewable, isMarkdownFile, Preview, type ConstructPreview } from '../services/preview'
import type { AppFile, AppLink } from 'types'

/**
 * Preview is abstract — the feature coverage lives in its concrete methods
 * (isMarkdown / previewType / isPdf / isImage / isVideo). We subclass it
 * minimally here so we can exercise those without pulling in FilePreview's
 * Pinia store dependency.
 */
class TestPreview extends Preview {
  constructor(data: Omit<ConstructPreview<Preview>, 'chunks'>) {
    super(data)
  }
}

/**
 * Build an AppFile with just the fields isFilePreviewable / isMarkdownFile
 * look at. The rest are filled with safe defaults so TS is happy.
 */
function makeFile(partial: Partial<AppFile>): AppFile {
  return {
    id: 'f1',
    user_id: 'u1',
    is_owner: true,
    name_hash: 'hash',
    name: 'file.bin',
    mime: 'application/octet-stream',
    size: 100,
    chunks: 1,
    encrypted_key: '',
    encrypted_name: '',
    cipher: 'aegis-128l',
    editable: false,
    active_version: 1,
    file_modified_at: 0,
    created_at: 0,
    is_new: false,
    ...partial
  } as AppFile
}

describe('isMarkdownFile', () => {
  it('UNIT: detects text/markdown by mime', () => {
    const file = makeFile({ name: 'notes.bin', mime: 'text/markdown' })
    expect(isMarkdownFile(file)).toBe(true)
  })

  it('UNIT: detects text/x-markdown by mime', () => {
    const file = makeFile({ name: 'notes.bin', mime: 'text/x-markdown' })
    expect(isMarkdownFile(file)).toBe(true)
  })

  it('UNIT: detects .md by extension regardless of mime', () => {
    const file = makeFile({ name: 'README.md', mime: 'application/octet-stream' })
    expect(isMarkdownFile(file)).toBe(true)
  })

  it('UNIT: .MD with uppercase extension still counts', () => {
    const file = makeFile({ name: 'README.MD', mime: 'application/octet-stream' })
    expect(isMarkdownFile(file)).toBe(true)
  })

  it('UNIT: rejects non-markdown files', () => {
    expect(isMarkdownFile(makeFile({ name: 'x.png', mime: 'image/png' }))).toBe(false)
    expect(isMarkdownFile(makeFile({ name: 'data.json', mime: 'application/json' }))).toBe(false)
  })
})

describe('isPreviewable (files)', () => {
  it('UNIT: markdown by mime is previewable', () => {
    const file = makeFile({ name: 'notes.bin', mime: 'text/markdown', size: 10 })
    expect(isPreviewable(file)).toBe(true)
  })

  it('UNIT: text/x-markdown is previewable', () => {
    const file = makeFile({ name: 'notes.bin', mime: 'text/x-markdown', size: 10 })
    expect(isPreviewable(file)).toBe(true)
  })

  it('UNIT: files with a thumbnail are previewable regardless of mime', () => {
    const file = makeFile({ mime: 'application/octet-stream', thumbnail: 'thumb-blob' })
    expect(isPreviewable(file)).toBe(true)
  })

  it('UNIT: PDFs are previewable', () => {
    const file = makeFile({ mime: 'application/pdf' })
    expect(isPreviewable(file)).toBe(true)
  })

  it('UNIT: SVG/HEIC/HEIF are previewable', () => {
    expect(isPreviewable(makeFile({ mime: 'image/svg+xml' }))).toBe(true)
    expect(isPreviewable(makeFile({ mime: 'image/heic' }))).toBe(true)
    expect(isPreviewable(makeFile({ mime: 'image/heif' }))).toBe(true)
  })

  it('UNIT: video/* is previewable', () => {
    expect(isPreviewable(makeFile({ mime: 'video/mp4' }))).toBe(true)
  })

  it('UNIT: non-previewable binaries are rejected', () => {
    const file = makeFile({ mime: 'application/octet-stream' })
    expect(isPreviewable(file)).toBe(false)
  })

  it('UNIT: zero-byte files are not previewable even if mime matches', () => {
    const file = makeFile({ mime: 'text/markdown', size: 0 })
    expect(isPreviewable(file)).toBe(false)
  })
})

describe('isPreviewable (links)', () => {
  function makeLink(partial: Partial<AppLink>): AppLink {
    return {
      id: 'l1',
      link_key_hashed: '',
      signature: '',
      file_id: 'f1',
      file_mime: 'application/pdf',
      file_size: 100,
      ...partial
    } as AppLink
  }

  it('UNIT: PDF link is previewable', () => {
    expect(isPreviewable(makeLink({ file_mime: 'application/pdf' }))).toBe(true)
  })

  it('UNIT: link with thumbnail is previewable', () => {
    expect(
      isPreviewable(makeLink({ file_mime: 'application/octet-stream', thumbnail: 'thumb' }))
    ).toBe(true)
  })

  it('UNIT: non-pdf, non-thumbnail link is NOT previewable', () => {
    expect(isPreviewable(makeLink({ file_mime: 'application/octet-stream' }))).toBe(false)
  })
})

describe('Preview.previewType', () => {
  it('UNIT: returns "markdown" for text/markdown', () => {
    const p = new TestPreview({
      id: '1',
      name: 'doc.bin',
      mime: 'text/markdown',
      size: 10,
      editable: true
    })
    expect(p.previewType()).toBe('markdown')
  })

  it('UNIT: returns "markdown" for .md extension', () => {
    const p = new TestPreview({
      id: '1',
      name: 'readme.md',
      mime: 'application/octet-stream',
      size: 10,
      thumbnail: 'thumb',
      editable: false
    })
    expect(p.previewType()).toBe('markdown')
  })

  it('UNIT: returns "pdf" for application/pdf', () => {
    const p = new TestPreview({
      id: '1',
      name: 'doc.pdf',
      mime: 'application/pdf',
      size: 10,
      editable: false
    })
    expect(p.previewType()).toBe('pdf')
  })

  it('UNIT: returns "image" for image/* with previewable size', () => {
    const p = new TestPreview({
      id: '1',
      name: 'x.png',
      mime: 'image/png',
      size: 10,
      thumbnail: 'thumb',
      editable: false
    })
    expect(p.previewType()).toBe('image')
  })

  it('UNIT: returns "video" for video/*', () => {
    const p = new TestPreview({
      id: '1',
      name: 'v.mp4',
      mime: 'video/mp4',
      size: 10,
      editable: false
    })
    expect(p.previewType()).toBe('video')
  })

  it('UNIT: returns null for non-previewable mime', () => {
    const p = new TestPreview({
      id: '1',
      name: 'a.bin',
      mime: 'application/octet-stream',
      size: 10,
      editable: false
    })
    expect(p.previewType()).toBeNull()
  })
})

describe('Preview.editable propagation', () => {
  it('UNIT: editable=true is preserved through the constructor', () => {
    const p = new TestPreview({
      id: '1',
      name: 'doc.md',
      mime: 'text/markdown',
      size: 10,
      editable: true
    })
    expect(p.editable).toBe(true)
  })

  it('UNIT: editable defaults to false when omitted', () => {
    // editable is required on ConstructPreview — simulate the same
    // behaviour `new FilePreview(file, ...)` would get when the server
    // returns an old file row (field missing). `!!undefined === false`.
    const p = new TestPreview({
      id: '1',
      name: 'doc.md',
      mime: 'text/markdown',
      size: 10,
      editable: undefined as unknown as boolean
    })
    expect(p.editable).toBe(false)
  })

  it('UNIT: editable=false is preserved', () => {
    const p = new TestPreview({
      id: '1',
      name: 'doc.md',
      mime: 'text/markdown',
      size: 10,
      editable: false
    })
    expect(p.editable).toBe(false)
  })

  it('UNIT: editable flag is orthogonal to preview type', () => {
    const editableMd = new TestPreview({
      id: '1',
      name: 'doc.md',
      mime: 'text/markdown',
      size: 10,
      editable: true
    })
    const readOnlyMd = new TestPreview({
      id: '2',
      name: 'doc.md',
      mime: 'text/markdown',
      size: 10,
      editable: false
    })
    expect(editableMd.previewType()).toBe(readOnlyMd.previewType())
    expect(editableMd.editable).not.toBe(readOnlyMd.editable)
  })
})

describe('Preview.isMarkdown (instance)', () => {
  it('UNIT: matches on mime text/markdown', () => {
    const p = new TestPreview({
      id: '1',
      name: 'doc.bin',
      mime: 'text/markdown',
      size: 10,
      editable: true
    })
    expect(p.isMarkdown()).toBe(true)
  })

  it('UNIT: matches on mime text/x-markdown', () => {
    const p = new TestPreview({
      id: '1',
      name: 'doc.bin',
      mime: 'text/x-markdown',
      size: 10,
      editable: true
    })
    expect(p.isMarkdown()).toBe(true)
  })

  it('UNIT: matches on .md extension regardless of mime', () => {
    const p = new TestPreview({
      id: '1',
      name: 'readme.md',
      mime: 'application/octet-stream',
      size: 10,
      editable: false
    })
    expect(p.isMarkdown()).toBe(true)
  })

  it('UNIT: does not match unrelated extensions/mimes', () => {
    const p = new TestPreview({
      id: '1',
      name: 'a.png',
      mime: 'image/png',
      size: 10,
      editable: false
    })
    expect(p.isMarkdown()).toBe(false)
  })
})
