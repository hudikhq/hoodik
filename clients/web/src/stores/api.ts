import { getCsrf, getJwt } from './auth'

export type Query = {
  [key: string]: string | number | string[] | { [key: string]: string | number | string[] }
}

export type Headers = { [key: string]: string }

/**
 * Interface to represent all the request data we will be doing
 */
export interface Request<B> {
  method: string
  url: string
  body: B | undefined
  headers: Headers
  query: Query | undefined
}

/**
 * Interface for the response we will be returning
 * for each request
 */
export interface Response<B = unknown, R = unknown> {
  request: Request<B>
  status: number
  headers: Headers
  rawBody: string | undefined
  body: R | undefined
}

export type InnerValidationErrors = { [key: string]: string | InnerValidationErrors }

/**
 * Error class for response
 *
 * @class
 */
export class ErrorResponse<B> extends Error {
  validation: InnerValidationErrors | null
  description: string

  request: Request<B>
  status: number
  headers: Headers
  rawBody: string | undefined
  body: ApiError | undefined

  constructor(response: Response<B, ApiError>) {
    super(
      `Request '${response.request.method.toUpperCase()} ${
        response.request.url
      }' failed with status ${response.status}`
    )

    this.request = response.request
    this.status = response.status
    this.headers = response.headers
    this.rawBody = response.rawBody
    this.body = response.body

    this.description = this._description()
    this.validation = this._validation()
  }

  /**
   * Try to extract the message that backend has returned
   */
  _description(): string {
    return this.body?.message || 'Unknown error'
  }

  /**
   * Extract validation errors if any on the body
   */
  _validation(): { [key: string]: string } | null {
    if (this.status !== 422 || !this.body?.context) {
      return null
    }

    // Any other context
    if (typeof this.body.context === 'string') {
      return null
    }

    if (!this.body?.context?.errors || typeof this.body.context.errors !== 'object') {
      return null
    }

    const validationErrors = this.body?.context?.errors

    const compiledErrors: { [key: string]: string } = {}

    for (const key in validationErrors) {
      if (
        validationErrors[key].errors &&
        Array.isArray(validationErrors[key].errors) &&
        validationErrors[key].errors.length
      ) {
        compiledErrors[key as string] = validationErrors[key].errors.join(', ')
      }
    }

    return compiledErrors
  }
}

export interface ValidationErrorInnerObject {
  field: string
  errors: string[]
}

export interface ValidationErrorObject {
  errors: { [key: string]: ValidationErrorInnerObject }
}

export interface ApiError {
  status: number
  message: string
  context?: string | ValidationErrorObject
}

/**
 * Remove the ending slash from the url
 */
export function ensureNoEndingSlash(url: string): string {
  return url.endsWith('/') ? url.slice(0, url.length - 1) : url
}

/**
 * Return the CLIENT URL from environment variable
 */
export function getClientUrl(): string {
  return ensureNoEndingSlash(import.meta.env.APP_CLIENT_URL || import.meta.env.APP_URL)
}

/**
 * Return the API URL from environment variable or fallback to the client URL
 */
export function getApiUrl(): string {
  return ensureNoEndingSlash(import.meta.env.APP_URL || 'http://localhost:4554')
}

/**
 * Convert values into string to be placed into an url query
 */
export function toQueryValue(
  value: string | number | boolean | string[] | number[] | object
): string | void {
  if (typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') {
    return `${value}`
  }

  if (Array.isArray(value)) {
    return value.join(',')
  }

  if (typeof value === 'object') {
    return JSON.stringify(value)
  }
}

/**
 * Main class to handle the requests
 * @class
 */
export default class Api {
  /**
   * Make get request
   */
  static async get<R>(
    path: string,
    query?: Query,
    headers?: Headers
  ): Promise<Response<undefined, R>> {
    return new Api().make('get', path, query, undefined, headers)
  }

  /**
   * Make post request
   * @throws
   */
  static async post<B, R>(
    path: string,
    query?: Query,
    body?: B,
    headers?: Headers
  ): Promise<Response<B, R>> {
    return new Api().make('post', path, query, body, headers)
  }

  /**
   * Make put request
   * @throws
   */
  static async put<B, R>(
    path: string,
    query?: Query,
    body?: B,
    headers?: Headers
  ): Promise<Response<B, R>> {
    return new Api().make('put', path, query, body, headers)
  }

  /**
   * Make delete request
   * @throws
   */
  static async delete<R>(
    path: string,
    query?: Query,
    headers?: Headers
  ): Promise<Response<undefined, R>> {
    return new Api().make('get', path, query, undefined, headers)
  }

  /**
   * Main method to run the requests
   * @throws
   */
  async make<B, R>(
    method: 'get' | 'post' | 'put' | 'delete',
    path: string,
    query?: Query,
    body?: B,
    headers?: Headers
  ): Promise<Response<B, R>> {
    const url = Api.getUrlWithQuery(path, query)
    const _headers = Api.getHeaders(headers)

    if (body && typeof body === 'object' && !_headers['Content-Type']) {
      _headers['Content-Type'] = 'application/json'
    }

    const request: Request<B> = {
      method,
      url,
      body,
      query,
      headers: _headers
    }

    const fetchOptions: RequestInit = {
      cache: 'no-cache',
      credentials: 'omit',
      headers: request.headers,
      method,
      mode: 'cors',
      redirect: 'follow'
    }

    if (request.body instanceof Buffer) {
      fetchOptions.body = request.body
    } else if (request.body && typeof request.body === 'object') {
      fetchOptions.body = JSON.stringify(request.body)
    } else if (request.body && typeof request.body === 'string') {
      fetchOptions.body = request.body
    }

    const res = await fetch(decodeURIComponent(url), fetchOptions)

    const rawBody = await res.text()

    let responseBody: R | undefined

    try {
      responseBody = JSON.parse(rawBody)
    } catch (e) {
      /* empty */
    }

    const responseHeaders: Headers = {}
    res.headers.forEach((value: string, key: string) => (responseHeaders[key] = value))

    const response = {
      request,
      status: res.status,
      body: responseBody,
      rawBody,
      headers: responseHeaders
    }

    if (response.status >= 400) {
      throw new ErrorResponse(response as Response<B, ApiError>)
    }

    return response
  }

  /**
   * Prepare headers before sending the request
   */
  static getHeaders(headers?: Headers): Headers {
    headers = headers || {}

    if (getCsrf()) {
      headers['X-Csrf-Token'] = getCsrf() || ''
    }

    if (getJwt()) {
      headers['Authorization'] = `Bearer ${getJwt() || ''}`
    }

    return headers
  }

  /**
   * Format URL with the query, take in consideration that there could already
   * be a query present in the path, so try to take that into account also
   */
  static getUrlWithQuery(path: string, query?: Query) {
    const url = new URL(`${getApiUrl()}${path}`)

    if (query && typeof query === 'object') {
      for (const name in query) {
        const value = toQueryValue(query[name])

        if (value) {
          url.searchParams.append(name, encodeURIComponent(value))
        }
      }
    }

    return `${url}`
  }
}
