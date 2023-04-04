import { writable } from 'svelte/store';
import Api from './api';
import * as crypto from './cryptfns';

export interface Authenticated {
	user: User;
	session?: Session;
}

export interface User {
	id: number;
	email: string;
	secret?: string;
	pubkey: string;
	encrypted_private_key?: string;
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
	privateKey?: string;
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

	if (response.body?.user.encrypted_private_key) {
		crypto.rsa.decryptSecretAndSet(response.body?.user.encrypted_private_key, credentials.password);
	} else if (credentials.privateKey) {
		crypto.rsa.set(crypto.rsa.inputToKeypair(credentials.privateKey));
	} else {
		throw new Error('No encrypted secret key found on user from backend, not privateKey provided');
	}

	return response.body as Authenticated;
}

/**
 * Generates the keypair from the mnemonic and attempts to get the current user from backend
 * @throws
 */
export async function loginWithPrivateKey(mnemonic: string): Promise<Authenticated> {
	const keypair = crypto.rsa.inputToKeypair(mnemonic);

	crypto.rsa.set(keypair);

	return self();
}

/**
 * Attempt to decrypt the secret key and get the current user from backend
 * @throws
 */
export async function loginWithPin(pin: string): Promise<Authenticated> {
	const encryptedSecret = crypto.aes.getEncryptedSecret();

	if (!encryptedSecret) {
		throw new Error('No encrypted secret found');
	}

	return loginWithPrivateKey(crypto.aes.decrypt(encryptedSecret, pin));
}
