/**
 * Hold a worker's messages back until it says it is listening.
 *
 * The transfer workers are ES modules whose wasm import wraps the entire
 * module body — including the `onmessage` assignment — in an async
 * initializer. The browser starts delivering messages as soon as the worker
 * exists, so anything posted while the wasm is still compiling arrives with
 * no handler attached and is silently dropped. A caller awaiting a reply to
 * a dropped message waits forever.
 *
 * The worker posts `{ type: 'ready' }` as its last initialization step;
 * until that arrives, `postMessage` queues locally and flushes in order.
 */
export function gateUntilReady(worker: Worker): Worker {
  const queued: unknown[][] = []
  let live = false
  const post = worker.postMessage.bind(worker)

  const flushOnReady = (event: MessageEvent) => {
    if (event.data?.type !== 'ready') return
    worker.removeEventListener('message', flushOnReady)
    live = true
    for (const args of queued) {
      post(...(args as Parameters<Worker['postMessage']>))
    }
    queued.length = 0
  }
  worker.addEventListener('message', flushOnReady)

  worker.postMessage = ((...args: unknown[]) => {
    if (live) {
      post(...(args as Parameters<Worker['postMessage']>))
    } else {
      queued.push(args)
    }
  }) as Worker['postMessage']

  return worker
}
