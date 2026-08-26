import { describe, it, expect } from 'vitest'

import { bareDigests } from '../services/storage/reindex'

/**
 * The sweep must be safe to run twice: re-keying an already-keyed digest
 * writes HMAC(HMAC(digest)) into the column, which nothing can ever undo or
 * match again. Shape separates keyed (32 hex) from bare for every algorithm
 * except MD5, whose bare form shares the tag's shape — that one rides on its
 * siblings.
 */
describe('bareDigests', () => {
  const bareSha256 = 'a'.repeat(64)
  const keyedTag = 'b'.repeat(32)

  it('selects bare values and skips keyed ones by shape', () => {
    const digests = bareDigests({
      sha1: 'c'.repeat(40),
      sha256: bareSha256,
      blake2b: 'd'.repeat(128)
    })

    expect(digests.sha1).toBe('c'.repeat(40))
    expect(digests.sha256).toBe(bareSha256)
    expect(digests.blake2b).toBe('d'.repeat(128))
  })

  it('selects nothing from a row the sweep already keyed', () => {
    const digests = bareDigests({ md5: keyedTag, sha256: keyedTag })

    expect(Object.values(digests).filter(Boolean)).toHaveLength(0)
  })

  it('keys MD5 only alongside a bare sibling', () => {
    // A bare row: MD5 is bare too, and the siblings prove it.
    expect(bareDigests({ md5: 'e'.repeat(32), sha256: bareSha256 }).md5).toBe('e'.repeat(32))

    // After the sweep every sibling is keyed; the same 32-hex MD5 value is
    // now a tag and must not be keyed again.
    expect(bareDigests({ md5: keyedTag, sha256: keyedTag }).md5).toBeUndefined()
  })

  it('re-keys the siblings a note save left bare while skipping its keyed sha256', () => {
    // A pre-migration note edited before the sweep: the save keyed sha256,
    // but sha1/blake2b/md5 still hold what the old world wrote.
    const digests = bareDigests({
      md5: 'e'.repeat(32),
      sha1: 'c'.repeat(40),
      sha256: keyedTag,
      blake2b: 'd'.repeat(128)
    })

    expect(digests.sha256).toBeUndefined()
    expect(digests.sha1).toBe('c'.repeat(40))
    expect(digests.blake2b).toBe('d'.repeat(128))
    expect(digests.md5).toBe('e'.repeat(32))
  })

  it('treats a non-hex value of digest length as not a digest', () => {
    expect(bareDigests({ sha256: 'z'.repeat(64) }).sha256).toBeUndefined()
  })
})
