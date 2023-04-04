import * as crypto from './cryptfns/rsa';
import Cookies from 'js-cookie';

export type Query = {
	[key: string]: string | number | string[] | { [key: string]: string | number | string[] };
};

export type Headers = { [key: string]: string };

/**
 * Interface to represent all the request data we will be doing
 */
export interface Request<B> {
	method: string;
	url: string;
	body: B | undefined;
	headers: Headers;
	query: Query | undefined;
}

/**
 * Interface for the response we will be returning
 * for each request
 */
export interface Response<B = unknown, R = unknown> {
	request: Request<B>;
	status: number;
	headers: Headers;
	rawBody: string | undefined;
	body: R | undefined;
}

/**
 * Error class for response
 *
 * @class
 */
export class ErrorResponse<B, R> extends Error {
	request: Request<B>;
	status: number;
	headers: Headers;
	rawBody: string | undefined;
	body: R | undefined;

	constructor(response: Response<B, R>) {
		super(
			`Request '${response.request.method.toUpperCase()} ${
				response.request.url
			}' failed with status ${response.status}`
		);

		this.request = response.request;
		this.status = response.status;
		this.headers = response.headers;
		this.rawBody = response.rawBody;
		this.body = response.body;
	}
}

const csrfCookieName = 'X-CSRF-TOKEN';

/**
 * Remove the ending slash from the url
 */
export function ensureEndingSlash(url: string): string {
	return url.endsWith('/') ? url.slice(0, url.length - 1) : url;
}

/**
 * Return the CLIENT URL from environment variable
 */
export function getClientUrl(): string {
	return ensureEndingSlash(import.meta.env.APP_CLIENT_URL || 'http://localhost:4554');
}

/**
 * Return the API URL from environment variable or fallback to the client URL
 */
export function getApiUrl(): string {
	return ensureEndingSlash(import.meta.env.APP_API_URL || getClientUrl());
}

/**
 * Load the CSRF token from the cookie
 */
export function getCsrf(): string | null {
	return Cookies.get(csrfCookieName) || null;
}

/**
 * Format URL with the query, take in consideration that there could already
 * be a query present in the path, so try to take that into account also
 */
export function getUrlWithQuery(path: string, query?: Query) {
	const url = new URL(`${getApiUrl()}${path}`);

	if (query && typeof query === 'object') {
		for (const name in query) {
			const value = toQueryValue(query[name]);

			if (value) {
				url.searchParams.append(name, encodeURIComponent(value));
			}
		}
	}

	return `${url}`;
}

/**
 * Convert values into string to be placed into an url query
 */
export function toQueryValue(
	value: string | number | boolean | string[] | number[] | object
): string | void {
	if (typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') {
		return `${value}`;
	}

	if (Array.isArray(value)) {
		return value.join(',');
	}

	if (typeof value === 'object') {
		return JSON.stringify(value);
	}
}

/**
 * Gives out a rounded epoch timestamp in minutes
 */
export function getFlattenedTimestampMinutes(): string {
	const timestamp = parseInt(`${Date.now() / 1000}`);
	const flat = `${parseInt(`${timestamp / 60}`)}`;

	return flat;
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
		return new Api().make('get', path, query, undefined, headers);
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
		return new Api().make('post', path, query, body, headers);
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
		return new Api().make('put', path, query, body, headers);
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
		return new Api().make('get', path, query, undefined, headers);
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
		const url = getUrlWithQuery(path, query);

		const request: Request<B> = {
			method,
			url,
			body,
			query,
			headers: await this.getHeaders(headers)
		};

		const fetchOptions: RequestInit = {
			cache: 'no-cache',
			credentials: 'include',
			headers: request.headers,
			method,
			mode: 'cors',
			redirect: 'follow'
		};

		if (request.body && typeof request.body === 'object') {
			fetchOptions.body = JSON.stringify(request.body);
		} else if (request.body && typeof request.body === 'string') {
			fetchOptions.body = request.body;
		}

		const res = await fetch(decodeURIComponent(url), fetchOptions);

		const rawBody = await res.text();

		let responseBody: R | undefined;

		try {
			responseBody = JSON.parse(rawBody);
		} catch (e) {
			/* empty */
		}

		const responseHeaders: Headers = {};
		res.headers.forEach((value: string, key: string) => (responseHeaders[key] = value));

		const response = {
			request,
			status: res.status,
			body: responseBody,
			rawBody,
			headers: responseHeaders
		};

		if (response.status >= 400) {
			throw new ErrorResponse(response);
		}

		return response;
	}

	/**
	 * Prepare headers before sending the request
	 */
	async getHeaders(headers?: Headers): Promise<Headers> {
		headers = headers || {};
		headers['Content-Type'] = 'application/json';

		if (getCsrf()) {
			headers['X-Csrf-Token'] = getCsrf() || '';
		}

		try {
			const { publicKey, signature } = await crypto.sign(getFlattenedTimestampMinutes());
			const fingerprint = await crypto.getFingerprint(publicKey);

			headers['Authorization'] = `Signature ${signature}`;
			headers['X-Key-Fingerprint'] = `${fingerprint}`;
		} catch (e) {
			/**/
		}

		return headers;
	}
}
