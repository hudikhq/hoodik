import * as api from './api'
import * as auth from './auth'
import * as crypto from './cryptfns'
export { auth, crypto, api }
import { parseISO, format as f, formatDistanceStrict } from 'date-fns'
import type { WorkerErrorType } from '../types'
import type { ErrorResponse } from './api'
import { notify } from '@kyvg/vue3-notification'
import { i18n, currentDateFnsLocale, currentLocale } from '@/i18n'

const DATE_FORMAT = "yyyy-MM-dd'T'HH:mm:ss.SSSSSS"

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

/**
 * Async/Await setTimeout
 */
export function wait(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

/**
 * Get the OS of the user
 */
export function os() {
  const userAgent = window.navigator.userAgent.toLowerCase(),
    macosPlatforms = /(macintosh|macintel|macppc|mac68k|macos)/i,
    windowsPlatforms = /(win32|win64|windows|wince)/i,
    iosPlatforms = /(iphone|ipad|ipod)/i

  let os = 'unknown'

  if (macosPlatforms.test(userAgent)) {
    os = 'macos'
  } else if (iosPlatforms.test(userAgent)) {
    os = 'ios'
  } else if (windowsPlatforms.test(userAgent)) {
    os = 'windows'
  } else if (/android/.test(userAgent)) {
    os = 'android'
  } else if (!os && /linux/.test(userAgent)) {
    os = 'linux'
  }

  return os
}

/**
 * Simple way to generate random uuid4
 */
export function uuidv4() {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function (c) {
    /* eslint-disable */
    const r = (Math.random() * 16) | 0
    const v = c === 'x' ? r : (r & 0x3) | 0x8
    /* eslint-enable */
    return v.toString(16)
  })
}

/**
 * Takes the UTC date and creates a local date
 * @throws
 */
export function localDateFromUtcString(utc?: string | Date | number | null): Date {
  if (typeof utc === 'number') {
    utc = new Date(utc * 1000)
  }

  if (!utc || new Date(utc as string).toDateString() === 'Invalid Date') {
    throw new Error('Invalid date')
  }

  if (typeof utc === 'string') {
    const date = parseISO(`${utc}Z`)

    return date
  }

  return utc
}

/**
 * Takes the LOCAL date and creates an UTC date
 */
export function utcStringFromLocal(local?: string | Date | number): string {
  if (local instanceof Date) {
    local = new Date(local.getTime() + local.getTimezoneOffset() * 60000)
  }

  return format(local || new Date(), DATE_FORMAT)
}

/**
 * Make the format function bit more versatile
 * @throws
 */
export function format(date: Date | string | number, formatString?: string): string {
  if (typeof date === 'number') {
    date = new Date(date * 1000)
  }

  if (!date || typeof date === 'string') {
    date = localDateFromUtcString(date)
  }

  return f(date, formatString || DATE_FORMAT, { locale: currentDateFnsLocale() })
}

/**
 * Single point of doing the 'pretty' dates for the entire app
 */
export function formatPrettyDate(date: Date | string | number): string {
  if (typeof date === 'number') {
    date = new Date(date * 1000)
  }

  return format(date, 'MMM do yyyy, HH:mm')
}

/**
 * Render a unix timestamp as a relative phrase ("2 minutes ago",
 * "yesterday") when the event is recent, falling back to the absolute
 * pretty date for older events. Used by the audit log where the typical
 * reader cares about "did this just happen" first and the exact wall
 * clock time second.
 */
export function formatRelative(unixSeconds: number, now: number = Date.now() / 1000): string {
  const delta = Math.max(0, now - unixSeconds)
  if (delta < 60) return i18n.global.t('time.justNow')
  if (delta < 7 * 86400) {
    return formatDistanceStrict(new Date(unixSeconds * 1000), new Date(now * 1000), {
      addSuffix: true,
      locale: currentDateFnsLocale()
    })
  }
  return formatPrettyDate(unixSeconds)
}

function sized(value: number, unit: 'B' | 'KB' | 'MB' | 'GB'): string {
  const formatted = new Intl.NumberFormat(currentLocale(), {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2
  }).format(value)

  return `${formatted} ${i18n.global.t(`size.${unit}`)}`
}

/**
 * Format bytes to human readable string
 */
export function formatSize(b?: number | string, unit?: 'B' | 'KB' | 'MB' | 'GB'): string {
  if (unit) {
    const sizes = {
      B: b,
      KB: b ? (b as number) / 1024 : undefined,
      MB: b ? (b as number) / 1024 / 1024 : undefined,
      GB: b ? (b as number) / 1024 / 1024 / 1024 : undefined
    }

    if (typeof sizes[unit] !== 'undefined') {
      return sized(sizes[unit] as number, unit)
    }
  }

  if (b === undefined || b === null) {
    return `0 ${i18n.global.t('size.B')}`
  }

  if (typeof b === 'string') {
    b = parseInt(b)
  }

  if (b < 1024) {
    return sized(b, 'B')
  }

  const kb = b / 1024

  if (kb < 1024) {
    return sized(kb, 'KB')
  }

  const mb = b / 1024 / 1024

  if (mb < 1024) {
    return sized(mb, 'MB')
  }

  return sized(b / 1024 / 1024 / 1024, 'GB')
}

/**
 * Convert common errors into WorkerErrorType
 */
export function errorIntoWorkerError(error: Error | ErrorResponse<any> | string): WorkerErrorType {
  if ((error as ErrorResponse<any>).kind === 'ErrorResponse') {
    return (error as ErrorResponse<any>).intoWorkerError()
  }

  if (error instanceof Error) {
    return { context: error.message, stack: error.stack }
  }

  return { context: `${error}` }
}
