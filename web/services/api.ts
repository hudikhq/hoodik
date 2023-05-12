import type { WorkerErrorType } from '../types'

export type Query = {
  [key: string]: string | number | string[] | undefined | null | Query
}

export type Headers = { [key: string]: string }

export type ApiTransfer = { jwt?: string | null; csrf?: string | null; apiUrl?: string }

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
  kind: string = 'ErrorResponse'

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
   * Convert into a worker error
   */
  intoWorkerError(): WorkerErrorType {
    const context = this.validation || this.description

    return { context, stack: this.stack }
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
 * Try to get origin from window.location
 */
export function getOrigin(): string {
  const windowish = getWindowish()

  if (windowish) {
    return ensureNoEndingSlash(windowish.location.origin)
  }

  return '/'
}

/**
 * Try to get window or self
 */
export function getWindowish(): Window | null {
  try {
    if ('location' in window) {
      return window
    }

    if ('location' in self) {
      return self
    }
  } catch (e) {
    // do nothing
  }

  return null
}

/**
 * Return the CLIENT URL from environment variable
 */
export function getClientUrl(): string {
  return ensureNoEndingSlash(
    import.meta.env.APP_CLIENT_URL || import.meta.env.APP_URL || getOrigin()
  )
}

/**
 * Return the API URL from environment variable or fallback to the client URL
 */
export function getApiUrl(): string {
  return ensureNoEndingSlash(import.meta.env.APP_URL || getOrigin())
}

/**
 * Convert values into string to be placed into an url query
 */
export function toQueryValue(
  value: string | number | boolean | string[] | number[] | undefined | null | Query
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
  private apiUrl: string

  constructor({ apiUrl }: ApiTransfer = {}) {
    this.apiUrl = apiUrl || getApiUrl()
  }

  /**
   * Convert the inner data to json
   * to pass into the service worker.
   */
  toJson(): ApiTransfer {
    return { apiUrl: this.apiUrl }
  }

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
    return new Api().make('delete', path, query, undefined, headers)
  }

  /**
   * Make get request
   */
  async download(path: string, query?: Query): Promise<globalThis.Response> {
    const { request, fetchOptions } = Api.buildRequest(
      'get',
      path,
      query,
      undefined,
      undefined,
      this
    )

    return fetch(decodeURIComponent(request.url), fetchOptions)
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
    const { request, fetchOptions } = Api.buildRequest(method, path, query, body, headers, this)

    if (request.body instanceof Uint8Array) {
      fetchOptions.body = request.body
    } else if (request.body && typeof request.body === 'object') {
      fetchOptions.body = JSON.stringify(request.body)
    } else if (request.body && typeof request.body === 'string') {
      fetchOptions.body = request.body
    }

    const res = await fetch(decodeURIComponent(request.url), fetchOptions)

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

  static buildRequest<B>(
    method: 'get' | 'post' | 'put' | 'delete',
    path: string,
    query?: Query,
    body?: B,
    headers?: Headers,
    api?: Api
  ) {
    api = api || new Api()

    const url = api.getUrlWithQuery(path, query)
    const _headers = api.getHeaders(headers)

    if (query) {
      for (const key in query) {
        if (query[key] === undefined || query[key] === null) {
          delete query[key]
        }
      }
    }

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
      credentials: 'include',
      headers: request.headers,
      method,
      mode: 'cors',
      redirect: 'follow'
    }

    return { request, fetchOptions }
  }

  /**
   * Prepare headers before sending the request
   */
  getHeaders(headers?: Headers): Headers {
    return headers || {}
  }

  /**
   * Format URL with the query, take in consideration that there could already
   * be a query present in the path, so try to take that into account also
   */
  getUrlWithQuery(path: string, query?: Query) {
    const url = new URL(`${this.apiUrl}${path}`)

    if (query !== null && query !== undefined && typeof query === 'object') {
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
