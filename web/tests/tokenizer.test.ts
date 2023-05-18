import { describe, it, expect } from 'vitest'
import * as cryptfns from '../services/cryptfns'

describe('Converting names into tokens', () => {
  it('UNIT: Tokens: can convert string into valid tokens', async () => {
    const filenames = ['test.txt', 'IMG_123455.jpg', 'some-document.doc']

    for (const filename of filenames) {
      const tokens = cryptfns.stringToHashedTokens(filename)
      expect(tokens.length).toBeGreaterThan(0)
    }
  })
})
