import * as crypto from '../cryptfns';
import * as login from './login';
import * as register from './register';
import { navigate } from 'svelte-routing';
import Cookies from 'js-cookie';
export { login, register };

const CSRF_COOKIE_NAME = 'X-CSRF-TOKEN';
const JWT_TOKEN_COOKIE_NAME = 'JWT-TOKEN';

/**
 * Load the CSRF token from the cookie
 */
export function getCsrf(): string | null {
	return Cookies.get(CSRF_COOKIE_NAME) || null;
}

/**
 * Set the CSRF token into cookie
 */
export function setCsrf(csrf: string, expires: Date) {
	Cookies.set(CSRF_COOKIE_NAME, csrf, {
		path: '/',
		sameSite: 'strict',
		domain: import.meta.env.APP_COOKIE_DOMAIN,
		expires
	});
}

/**
 * Shortcut to figure out if we can make requests
 */
export function maybeCouldMakeRequests(): boolean {
	return !!getCsrf() && !!getJwt();
}

/**
 * Load the JWT token from the cookie
 */
export function getJwt(): string | null {
	return Cookies.get(CSRF_COOKIE_NAME) || null;
}

/**
 * Set the JWT token into cookie
 */
export function setJwt(jwt: string, expires: Date) {
	Cookies.set(JWT_TOKEN_COOKIE_NAME, jwt, {
		path: '/',
		sameSite: 'strict',
		domain: import.meta.env.APP_COOKIE_DOMAIN,
		expires
	});
}

/**
 * Do we have authentication currently loaded?
 */
export async function hasAuthentication() {
	return !!(await login.get());
}

/**
 * Lets us know should we even attempt at making the authentication getting request
 */
export async function maybeShouldAttemptSelf(): Promise<boolean> {
	if ((await hasAuthentication()) && maybeCouldMakeRequests()) {
		return false;
	}

	return maybeCouldMakeRequests();
}

/**
 * Ensure we have authentication and move user to appropriate pages if not
 */
export async function ensureAuthenticated(): Promise<void> {
	if (!(await maybeShouldAttemptSelf())) {
		if (crypto.hasEncryptedPrivateKey()) {
			return navigate('/auth/decrypt');
		}

		return navigate('/auth/login');
	}

	try {
		await login.self();
	} catch (e) {
		navigate('/auth/login');
	}
}
