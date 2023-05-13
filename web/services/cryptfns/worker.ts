import { encrypt as aesEncrypt, decrypt as aesDecrypt } from './aes'
import * as logger from '!/logger'
import { MAX_WORKERS, MAX_WAIT_FOR_WORKER_MS } from '!/storage/constants'

// @ts-ignore
import { serviceWorkerFile } from 'virtual:vite-plugin-service-worker'
import { uuidv4 } from '..'

let __triedSetup = false
let lastUsed = 0

/**
 * Pool of workers ready to receive jobs
 */
const __workers: Worker[] = []

/**
 * Results from the workers ready to be picked up
 * and sent back to their respective callers
 */
const __results = new Map<string, Uint8Array>()

/**
 * Try getting the worker if we are in position
 * to have a web worker available to us.
 */
async function getWorker(): Promise<number | undefined> {
  if (__workers.length >= MAX_WORKERS) {
    if (lastUsed < __workers.length) {
      const index = lastUsed
      lastUsed++
      return index
    }

    lastUsed = 0
    return 0
  }

  if (__triedSetup) {
    return
  }

  __triedSetup = true

  return new Promise((resolve) => {
    const fn = async () => {
      return await getWorker()
    }

    try {
      if ('Worker' in window) {
        setupWorkers().then(fn).then(resolve)
      }
    } catch (e) {
      // probably no worker support, or no window
    }

    try {
      if ('Worker' in self) {
        setupWorkers().then(fn).then(resolve)
      }
    } catch (e) {
      // probably no worker support, or no self
    }
  })
}

/**
 * Setup the workers pool
 */
async function setupWorkers() {
  for (let i = 0; i < MAX_WORKERS; i++) {
    await setupSingleWorker(i)
  }
}

/**
 * Setup single worker in the pool, send a ping message
 * and do a little timeout for it to set it self up.
 */
async function setupSingleWorker(index: number): Promise<void> {
  return new Promise((resolve) => {
    const worker = new Worker(serviceWorkerFile, {
      type: 'module',
      name: `Hoodik Crypto Worker ${index}`
    })

    worker.onmessage = async (event) => {
      if (event.data.type === 'encrypt' || event.data.type === 'decrypt') {
        logger.debug(
          "Worker's response, returned to main thread",
          event.data.type,
          event.data.message.id
        )

        __results.set(event.data.message.id, event.data.message.result)
      } else {
        logger.debug("Worker's response, returned to main thread", event.data.type)
      }
    }

    worker.onerror = (event) => {
      logger.error("Worker's error, returned to main thread", event)
    }

    setTimeout(() => {
      worker.postMessage({ type: 'ping' })
      __workers.push(worker)

      setTimeout(() => {
        resolve()
      }, 100)
    }, 100)
  })
}

/**
 * Post message to the worker, and return the result once the worker responds
 */
function postMessage(
  index: number,
  type: string,
  data: Uint8Array,
  key: Uint8Array
): Promise<Uint8Array> {
  return new Promise((resolve, reject) => {
    if (!__workers[index]) {
      return reject(new Error('Worker not found'))
    }

    const id = uuidv4()
    const started = new Date().getTime()

    logger.debug('Sending message to the worker', index, type, id)

    __workers[index].postMessage({ type, message: { data, key, id } })

    const interval = setInterval(() => {
      if (__results.has(id)) {
        const result = __results.get(id)
        __results.delete(id)

        clearInterval(interval)

        resolve(result as Uint8Array)
      }

      const now = new Date().getTime()

      if (now - started > MAX_WAIT_FOR_WORKER_MS) {
        clearInterval(interval)
        reject(new Error('Timeout waiting for worker to respond'))
      }
    }, 1)
  })
}

/**
 * Encrypt data with the selected key,
 * attempt to offload it to the web worker, if it fails, use the main thread
 */
export async function encrypt(plaintext: Uint8Array, key: Uint8Array): Promise<Uint8Array> {
  const index = await getWorker()

  if (typeof index !== 'undefined') {
    return postMessage(index, 'encrypt', plaintext, key)
  }

  return aesEncrypt(plaintext, key)
}

/**
 * Encrypt data with the selected key,
 * attempt to offload it to the web worker, if it fails, use the main thread
 */
export async function decrypt(ciphertext: Uint8Array, key: Uint8Array): Promise<Uint8Array> {
  const worker = await getWorker()

  if (worker) {
    return postMessage(worker, 'decrypt', ciphertext, key)
  }

  return aesDecrypt(ciphertext, key)
}
