import { describe, expect, it } from 'vitest'
import { prettyMime } from '../src/utils/mime'

describe('prettyMime', () => {
  it('labels directories as folders', () => {
    expect(prettyMime('dir')).toBe('Folder')
  })

  it('labels media by top-level type', () => {
    expect(prettyMime('image/jpeg')).toBe('Image')
    expect(prettyMime('video/mp4')).toBe('Video')
    expect(prettyMime('audio/mpeg')).toBe('Audio')
    expect(prettyMime('text/markdown')).toBe('Text')
    expect(prettyMime('font/woff2')).toBe('Font')
  })

  it('labels well-known application subtypes', () => {
    expect(prettyMime('application/pdf')).toBe('PDF')
    expect(prettyMime('application/zip')).toBe('Archive')
    expect(
      prettyMime('application/vnd.openxmlformats-officedocument.spreadsheetml.sheet')
    ).toBe('Spreadsheet')
  })

  it('falls back to the raw mime for unknown types', () => {
    expect(prettyMime('application/x-custom-thing')).toBe('application/x-custom-thing')
  })

  it('returns an empty string for missing values', () => {
    expect(prettyMime(undefined)).toBe('')
    expect(prettyMime(null)).toBe('')
  })
})
