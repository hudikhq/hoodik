import * as api from './api'
import * as auth from './auth'
import * as crypto from './cryptfns'
export { auth, crypto, api }
import { parseISO, format as f } from 'date-fns'
import type { WorkerErrorType } from './types'
import type { ErrorResponse } from './api'

const DATE_FORMAT = "yyyy-MM-dd'T'HH:mm:ss.SSSSSS"

/**
 * Takes the UTC date and creates a local date
 * @throws
 */
export function localDateFromUtcString(utc?: string | Date | null): Date {
  if (!utc || new Date(utc as string).toDateString() === 'Invalid Date') {
    throw new Error('Invalid date')
  }

  if (typeof utc === 'string') {
    return parseISO(`${utc}Z`)
  }

  return utc
}

/**
 * Takes the LOCAL date and creates an UTC date
 */
export function utcStringFromLocal(local?: string | Date): string {
  return format(local || new Date(), DATE_FORMAT)
}

/**
 * Make the format function bit more versatile
 * @throws
 */
export function format(date: Date | string, formatString?: string): string {
  if (!date || typeof date === 'string') {
    date = localDateFromUtcString(date)
  }

  date = new Date(date.getTime() + date.getTimezoneOffset() * 60000)

  return f(date, formatString || DATE_FORMAT)
}

/**
 * Single point of doing the 'pretty' dates for the entire app
 */
export function formatPrettyDate(date: Date | string): string {
  return format(date, 'MMM do yyyy, hh:mm')
}

/**
 * Format bytes to human readable string
 */
export function formatSize(b?: number | string): string {
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
