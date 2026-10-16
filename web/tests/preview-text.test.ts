import { describe, it, expect } from 'vitest'
import {
  isTextFile,
  decodeText,
  TEXT_PREVIEW_MAX_BYTES,
  TEXT_PREVIEW_MAX_LINES
} from '../services/preview/text'

const encode = (s: string) => new TextEncoder().encode(s)

describe('isTextFile', () => {
  it('UNIT: accepts any text/* mime', () => {
    for (const mime of ['text/plain', 'text/csv', 'text/html', 'text/x-python', 'text/plain; charset=utf-8']) {
      expect(isTextFile({ name: 'x', mime })).toBe(true)
    }
  })

  it('UNIT: accepts whitelisted application mimes and structured suffixes', () => {
    for (const mime of [
      'application/json',
      'application/yaml',
      'application/x-yaml',
      'application/xml',
      'application/toml',
      'application/javascript',
      'application/x-sh',
      'application/sql',
      'application/manifest+json',
      'application/atom+xml'
    ]) {
      expect(isTextFile({ name: 'x', mime })).toBe(true)
    }
  })

  it('UNIT: falls back to the extension when the browser gave no mime', () => {
    for (const name of ['config.yaml', 'main.rs', 'Cargo.TOML', 'app.go', 'server.log', 'a.tsx']) {
      expect(isTextFile({ name, mime: 'application/octet-stream' })).toBe(true)
    }
  })

  it('UNIT: recognises conventional extensionless and dot files', () => {
    for (const name of ['Dockerfile', 'Makefile', 'LICENSE', '.gitignore', '.env', '.env.local']) {
      expect(isTextFile({ name, mime: 'application/octet-stream' })).toBe(true)
    }
  })

  it('UNIT: rejects binaries and unknown files', () => {
    expect(isTextFile({ name: 'a.zip', mime: 'application/zip' })).toBe(false)
    expect(isTextFile({ name: 'a.bin', mime: 'application/octet-stream' })).toBe(false)
    expect(isTextFile({ name: 'noextension', mime: 'application/octet-stream' })).toBe(false)
    expect(isTextFile({ name: 'trailingdot.', mime: '' })).toBe(false)
    expect(isTextFile({ name: 'x.png', mime: 'image/png' })).toBe(false)
    expect(isTextFile({ name: 'x.svg', mime: 'image/svg+xml' })).toBe(false)
  })
})

describe('decodeText', () => {
  it('UNIT: splits on every newline style and drops the trailing empty line', () => {
    const out = decodeText(encode('a\r\nb\rc\nd\n'))
    expect(out).toEqual({ lines: ['a', 'b', 'c', 'd'], truncated: false, binary: false })
  })

  it('UNIT: keeps blank lines in the middle', () => {
    expect(decodeText(encode('a\n\n\nb')).lines).toEqual(['a', '', '', 'b'])
  })

  it('UNIT: an empty file is one empty line', () => {
    expect(decodeText(new Uint8Array()).lines).toEqual([''])
  })

  it('UNIT: strips a UTF-8 BOM', () => {
    const bytes = new Uint8Array([0xef, 0xbb, 0xbf, ...encode('héllo')])
    expect(decodeText(bytes).lines).toEqual(['héllo'])
  })

  it('UNIT: decodes UTF-16LE with a BOM instead of calling it binary', () => {
    const bytes = new Uint8Array([0xff, 0xfe, 0x68, 0x00, 0x69, 0x00, 0x0a, 0x00, 0x21, 0x00])
    expect(decodeText(bytes)).toEqual({ lines: ['hi', '!'], truncated: false, binary: false })
  })

  it('UNIT: flags NUL bytes as binary', () => {
    const out = decodeText(new Uint8Array([0x68, 0x00, 0x69]))
    expect(out.binary).toBe(true)
    expect(out.lines).toEqual([])
  })

  it('UNIT: a partial download is truncated and loses its cut-off last line', () => {
    const out = decodeText(encode('one\ntwo\nthr'), true)
    expect(out).toEqual({ lines: ['one', 'two'], truncated: true, binary: false })
  })

  it('UNIT: caps the decoded bytes without breaking a multi-byte character', () => {
    // A single line is kept even when cut. The leading 1-byte 'a' shifts the
    // 2-byte characters so the cap lands in the middle of one.
    const out = decodeText(encode('a' + 'é'.repeat(TEXT_PREVIEW_MAX_BYTES)))
    expect(out.truncated).toBe(true)
    expect(out.lines).toEqual(['a' + 'é'.repeat((TEXT_PREVIEW_MAX_BYTES - 2) / 2)])
  })

  it('UNIT: caps the number of lines', () => {
    const out = decodeText(encode('x\n'.repeat(TEXT_PREVIEW_MAX_LINES + 10)))
    expect(out.truncated).toBe(true)
    expect(out.lines.length).toBe(TEXT_PREVIEW_MAX_LINES)
  })
})
