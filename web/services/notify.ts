import { notify } from '@kyvg/vue3-notification'
import { i18n } from '@/i18n'
import type { ErrorResponse } from './api'

/**
 * These live apart from `services/index.ts` on purpose: that barrel pulls in
 * the crypto and auth layers, and reporting an error is something even the
 * smallest component needs to do without dragging WASM into its bundle.
 */

/**
 * Turn whatever was thrown into something a person can act on.
 *
 * fetch rejects with a bare TypeError when the request never reached the
 * server, whose message ("Failed to fetch", "Load failed", "NetworkError…")
 * is browser-specific and means nothing to a user — so that case gets named
 * explicitly rather than passed through.
 */
export function humanizeError(error: unknown): string {
  if (typeof error === 'string' && error) {
    return error
  }

  if ((error as ErrorResponse<any>)?.kind === 'ErrorResponse') {
    return (error as ErrorResponse<any>).description || i18n.global.t('errors.unknown')
  }

  const message = (error as Error)?.message

  if (!message) {
    return i18n.global.t('errors.unknown')
  }

  const offline = typeof navigator !== 'undefined' && navigator.onLine === false
  const looksLikeNetwork =
    error instanceof TypeError || /failed to fetch|load failed|networkerror/i.test(message)

  if (offline || looksLikeNetwork) {
    return i18n.global.t('errors.network')
  }

  // Errors thrown inside the client carry a code the same way the server's do.
  // Translate it when the dictionary knows it, so an internal identifier never
  // reaches the screen just because it was raised locally instead of remotely.
  const key = `errors.${message.split(':')[0]}`
  if (i18n.global.te(key)) {
    return i18n.global.t(key)
  }

  return message
}

/**
 * Unify way to handle every kind of error as an error notification
 */
export function errorNotification(error: string | Error | ErrorResponse<any> | unknown) {
  if (typeof error === 'string') {
    return notification(error || i18n.global.t('errors.unknown'), undefined, 'error')
  }

  notification(i18n.global.t('errors.requestFailed'), humanizeError(error), 'error')
}

/**
 * Regular notification sender
 */
export function notification(
  title: string,
  text?: string,
  type: 'success' | 'error' | 'info' = 'info'
) {
  notify({
    type,
    title,
    text
  })
}
