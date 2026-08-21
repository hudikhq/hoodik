import { describe, it, expect } from 'vitest'

import { gateUntilReady } from '../services/worker-gate'

/**
 * The transfer workers attach their message handler only after their wasm
 * import finishes compiling, and the browser drops anything delivered before
 * that. These pin the guard: messages posted before the worker's `ready`
 * announcement must be held and delivered in order afterwards, not lost.
 */

type Listener = (event: { data: unknown }) => void

class FakeWorker {
  posted: unknown[] = []
  private listeners: Listener[] = []

  postMessage(message: unknown) {
    this.posted.push(message)
  }

  addEventListener(_type: string, listener: Listener) {
    this.listeners.push(listener)
  }

  removeEventListener(_type: string, listener: Listener) {
    this.listeners = this.listeners.filter((l) => l !== listener)
  }

  emit(data: unknown) {
    for (const listener of [...this.listeners]) {
      listener({ data })
    }
  }
}

function gated() {
  const fake = new FakeWorker()
  return { fake, worker: gateUntilReady(fake as unknown as Worker) }
}

describe('gateUntilReady', () => {
  it('holds messages until ready, then flushes them in order', () => {
    const { fake, worker } = gated()

    worker.postMessage({ type: 'download-bytes', message: { request: 'a' } })
    worker.postMessage({ type: 'ping' })
    expect(fake.posted).toEqual([])

    fake.emit({ type: 'ready' })
    expect(fake.posted).toEqual([
      { type: 'download-bytes', message: { request: 'a' } },
      { type: 'ping' }
    ])
  })

  it('passes messages straight through once ready', () => {
    const { fake, worker } = gated()

    fake.emit({ type: 'ready' })
    worker.postMessage({ type: 'ping' })
    expect(fake.posted).toEqual([{ type: 'ping' }])
  })

  it('ignores other messages while waiting', () => {
    const { fake, worker } = gated()

    worker.postMessage({ type: 'ping' })
    fake.emit({ type: 'pong' })
    expect(fake.posted).toEqual([])

    fake.emit({ type: 'ready' })
    expect(fake.posted).toEqual([{ type: 'ping' }])
  })
})
