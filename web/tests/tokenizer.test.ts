import { describe, it, expect } from 'vitest'
import * as cryptfns from '../services/cryptfns'

describe('Converting names into tokens', () => {
  const key = cryptfns.searchFileKey(new Uint8Array(32).fill(7))

  it('UNIT: Tokens: can convert string into valid tagged tokens', async () => {
    const filenames = ['test.txt', 'IMG_123455.jpg', 'some-document.doc']

    for (const filename of filenames) {
      const tokens = cryptfns.searchTags(key, filename)

      expect(tokens.length).toBeGreaterThan(0)
      for (const token of tokens) {
        expect(token).toMatch(/^[0-9a-f]{32}:\d+$/)
      }
    }
  })

  it('UNIT: Tokens: a different key tags the same name differently', async () => {
    const other = cryptfns.searchFileKey(new Uint8Array(32).fill(9))

    expect(cryptfns.searchTags(key, 'invoice.pdf')).not.toEqual(
      cryptfns.searchTags(other, 'invoice.pdf')
    )
  })

  it('UNIT: Tokens: a tag is never the token\'s bare digest', async () => {
    const [tag] = cryptfns.searchTags(key, 'invoice')

    expect(tag.split(':')[0]).not.toBe(cryptfns.sha256.digest('invoice'))
  })
})
