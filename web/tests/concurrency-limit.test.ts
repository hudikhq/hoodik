import { describe, expect, it } from 'vitest'

import { concurrencyLimit } from '../services'

function deferred() {
  let resolve!: () => void
  let reject!: (err: Error) => void
  const promise = new Promise<void>((res, rej) => {
    resolve = res
    reject = rej
  })
  return { promise, resolve, reject }
}

describe('concurrencyLimit', () => {
  it('UNIT: never runs more than the limit at once', async () => {
    const pool = concurrencyLimit(3)
    let active = 0
    let peak = 0

    for (let i = 0; i < 20; i++) {
      await pool.run(async () => {
        active++
        peak = Math.max(peak, active)
        await new Promise((r) => setTimeout(r, 1))
        active--
      })
    }
    await pool.settled()

    expect(peak).toBe(3)
    expect(active).toBe(0)
  })

  it('UNIT: run resolves once the task starts, not when it finishes', async () => {
    const pool = concurrencyLimit(2)
    const first = deferred()
    let finished = false

    await pool.run(async () => {
      await first.promise
      finished = true
    })

    expect(finished).toBe(false)

    first.resolve()
    await pool.settled()

    expect(finished).toBe(true)
  })

  it('UNIT: a task that throws frees its slot', async () => {
    const pool = concurrencyLimit(1)
    const failing = deferred()
    let ranAfter = false

    await pool.run(() => failing.promise)
    const next = pool.run(async () => {
      ranAfter = true
    })

    failing.reject(new Error('boom'))
    await next
    await pool.settled()

    expect(ranAfter).toBe(true)
  })
})
