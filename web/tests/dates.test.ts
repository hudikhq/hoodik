import { describe, it, expect } from 'vitest'
import * as main from '../services'

const EXAMPLE_UTC = '2023-04-07T17:28:17.000000'
const TIMESTAMP = 1686854683

describe('Working with dates', () => {
  it('UNIT: Date: just print out dates to figure out everything is okay', async () => {
    const date = new Date(TIMESTAMP * 1000)
    const parsedDate = main.localDateFromUtcString(TIMESTAMP)

    expect(parsedDate.valueOf()).toBe(date.valueOf())

    const local = main.localDateFromUtcString(EXAMPLE_UTC)
    const localString = main.utcStringFromLocal(local)

    expect(localString).toBe(EXAMPLE_UTC)

    try {
      main.localDateFromUtcString('whatever nonsense')
      expect(false).toBe(true)
    } catch (e) {
      expect(e.message).toBe('Invalid date')
    }
  })
})
