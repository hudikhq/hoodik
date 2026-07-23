import * as logger from './logger'

// @ts-ignore
import { serviceWorkerFile } from 'virtual:vite-plugin-service-worker'

/**
 * Spawn the transfer workers on first queue start instead of at boot.
 * Each transfer worker compiles the multi-megabyte crypto WASM; doing
 * that during boot raced first paint on three threads at once. By queue
 * start the UI is already mounted and interactive.
 *
 * Lives in its own module, imported dynamically behind a `Worker`
 * capability check — the worker URLs only resolve under Vite, and
 * nothing test environments load should depend on them.
 */
export function ensureWorkers(): void {
  try {
    if (!window.UPLOAD) {
      logger.debug('[queue] Creating UPLOAD worker from', serviceWorkerFile)
      window.UPLOAD = new Worker(serviceWorkerFile, { type: 'module', name: 'Hoodik Upload Worker' })
    }

    if (!window.DOWNLOAD) {
      logger.debug('[queue] Creating DOWNLOAD worker from', serviceWorkerFile)
      window.DOWNLOAD = new Worker(serviceWorkerFile, {
        type: 'module',
        name: 'Hoodik Download Worker'
      })
    }

    if (!window.HASH) {
      logger.debug('[queue] Creating HASH worker')
      window.HASH = new Worker(new URL('../hash-worker.ts', import.meta.url), {
        type: 'module',
        name: 'Hoodik Hash Worker'
      })
    }
  } catch (error) {
    logger.error('[queue] Worker creation failed:', error)
  }
}
