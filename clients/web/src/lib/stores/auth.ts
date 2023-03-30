import { writable } from 'svelte/store';
import Api from './api';
import * as crypto from './crypto';

export interface Authenticated {
	user: User;
	session?: Session;
}

export interface User {
	id: number;
	email: string;
	secret?: string;
	pubkey: string;
	encrypted_secret_key?: string;
	created_at: Date;
	updated_at: Date;
}

export interface Session {
	id: number;
	user_id: number;
	token: string;
	csrf: string;
	created_at: Date;
	updated_at: Date;
	expires_at: Date;
}

export interface Credentials {
	email: string;
	password: string;
	token?: string;
	remember?: boolean;
	mnemonic?: string;
}

export const { subscribe, set: _set } = writable<Authenticated | null>();

/**
 * Set Authenticated object
 */
export function set(auth: Authenticated) {
	_set(auth);
}

/**
 * Gets the authenticated object if it exists
 */
export function get(): Promise<Authenticated | null> {
	return new Promise((resolve) => subscribe(resolve));
}

/**
 * Clear Authenticated object
 */
export function clear() {
	_set(null);
}

/**
 * Try to get the current user
 * @throws
 */
export async function self(): Promise<Authenticated> {
	const response = await Api.post<undefined, Authenticated>('/api/auth/self');

	set(response.body as Authenticated);

	return response.body as Authenticated;
}

/**
 * Perform login operation regularly with normal credentials
 * @throws
 */
export async function login(credentials: Credentials): Promise<Authenticated> {
	const response = await Api.post<Credentials, Authenticated>(
		'/api/auth/login',
		undefined,
		credentials
	);

	set(response.body as Authenticated);

	if (response.body?.user.encrypted_secret_key) {
		crypto.decryptAndSet(response.body?.user.encrypted_secret_key, credentials.password);
	} else if (credentials.mnemonic) {
		crypto.set(crypto.generateKeyFrom(credentials.mnemonic));
	} else {
		throw new Error('No encrypted secret key found on user from backend, not mnemonic provided');
	}

	return response.body as Authenticated;
}

/**
 * Generates the keypair from the mnemonic and attempts to get the current user from backend
 * @throws
 */
export async function loginWithMnemonic(mnemonic: string): Promise<Authenticated> {
	const keypair = crypto.generateKeyFrom(mnemonic);

	crypto.set(keypair);

	return self();
}

/**
 * Attempt to decrypt the secret key and get the current user from backend
 * @throws
 */
export async function loginWithPin(pin: string): Promise<Authenticated> {
	const encryptedSecret = crypto.getEncryptedSecret();

	if (!encryptedSecret) {
		throw new Error('No encrypted secret found');
	}

	return loginWithMnemonic(crypto.decrypt(encryptedSecret, pin));
}
