import { writable } from 'svelte/store';
import type { AuthenticatedJwt } from './login';
import * as crypto from '../cryptfns';
import Api from '../api';

export interface CreateUser {
	email: string;
	password: string;
	secret?: string;
	token?: string;
	pubkey: string;
	fingerprint: string;
	encrypted_private_key?: string;

	/**
	 * Optional parameters that are only used for the registration process
	 * on the frontend. Private key is not sent to backend server unencrypted
	 */
	unencrypted_private_key?: string;
	confirm_password?: string;
	checkbox?: boolean;
	store_private_key?: boolean;
	i_have_stored_my_private_key?: boolean;
}

export const { subscribe, set: _set } = writable<Partial<CreateUser>>({});

/**
 * Set CreateUser object
 */
export async function set(data: Partial<CreateUser>) {
	const createUser = await get();
	_set({ ...createUser, ...data });
}

/**
 * Gets the CreateUser object if it exists
 */
export function get(): Promise<Partial<CreateUser>> {
	return new Promise(subscribe);
}

/**
 * Clear CreateUser object
 */
export function clear() {
	_set({});
}

/**
 * Make post request to create new user
 * @throws
 */
export async function postRegistration(data: CreateUser): Promise<AuthenticatedJwt> {
	const response = await Api.post<CreateUser, AuthenticatedJwt>(
		'/api/auth/register',
		undefined,
		data
	);

	return response.body as AuthenticatedJwt;
}

/**
 * Generate keypair and register new user
 * @throws
 */
export async function register(data: CreateUser): Promise<AuthenticatedJwt> {
	if (data.unencrypted_private_key && data.store_private_key) {
		data.encrypted_private_key = await crypto.rsa.protectPrivateKey(
			data.unencrypted_private_key as string,
			data.password as string
		);

		// Remove the key from the request payload
		delete data.unencrypted_private_key;
	}

	return postRegistration(data as CreateUser);
}
