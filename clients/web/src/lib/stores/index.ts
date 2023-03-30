import * as api from './api';
import * as auth from './auth';
import * as crypto from './crypto';
import * as register from './register';

import { navigate } from 'svelte-routing';

export { auth, crypto, api, register };

/**
 * Do we have authentication currently loaded?
 */
export async function hasAuthentication() {
	return !!(await auth.get());
}

/**
 * Verify that we can make authenticated requests on our backend
 *
 * Note: this is still not a 100% sure way to know that we can and don't blindly trust it
 */
export async function canMakeAuthenticatedRequests(): Promise<boolean> {
	if (api.getCsrf()) {
		return true;
	}

	try {
		return !!(await crypto.get());
	} catch (e) {
		return false;
	}
}

/**
 * Lets us know should we even attempt at making the authentication getting request
 */
export async function shouldAttemptSelf(): Promise<boolean> {
	if (await hasAuthentication()) {
		return false;
	}

	return canMakeAuthenticatedRequests();
}

/**
 * Ensure we have authentication and move user to appropriate pages if not
 */
export async function ensureAuthenticated(): Promise<void> {
	if (!(await shouldAttemptSelf())) {
		if (crypto.hasEncryptedSecretKey()) {
			return navigate('/auth/decrypt');
		}

		return navigate('/auth/login');
	}

	try {
		await auth.self();
	} catch (e) {
		navigate('/auth/login');
	}
}
