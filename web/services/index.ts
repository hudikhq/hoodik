import * as api from './api'
import * as auth from './auth'
import * as crypto from './cryptfns'
export { auth, crypto, api }
import { parseISO, format as f } from 'date-fns'
import type { WorkerErrorType } from '../types'
import type { ErrorResponse } from './api'

const DATE_FORMAT = "yyyy-MM-dd'T'HH:mm:ss.SSSSSS"

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

  return f(date, formatString || DATE_FORMAT)
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
      return `${(sizes[unit] as number).toFixed(2)} ${unit}`
    }
  }

  if (b === undefined || b === null) {
    return '0 B'
  }

  if (typeof b === 'string') {
    b = parseInt(b)
  }

  if (b < 1024) {
    return `${b.toFixed(2)} B`
  }

  const kb = b / 1024

  if (kb < 1024) {
    return `${kb.toFixed(2)} KB`
  }

  const mb = b / 1024 / 1024

  if (mb < 1024) {
    return `${mb.toFixed(2)} MB`
  }

  const gb = b / 1024 / 1024 / 1024

  return `${gb.toFixed(2)} GB`
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
