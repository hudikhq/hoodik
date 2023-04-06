import { writable } from 'svelte/store';
import Api from '../api';
import * as crypto from '../cryptfns';
import { local } from '../';
import { setJwt, setCsrf } from './';

export interface Authenticated {
	user: User;
	session: Session;
}

export interface AuthenticatedJwt {
	authenticated: Authenticated;
	jwt: string;
}

export interface User {
	id: number;
	email: string;
	private?: string;
	pubkey: string;
	fingerprint: string;
	encrypted_private_key?: string;
	created_at: string;
	updated_at: string;
}

export interface Session {
	id: number;
	user_id: number;
	token: string;
	csrf: string;
	created_at: string;
	updated_at: string;
	expires_at: string;
}

export interface Credentials {
	email: string;
	password: string;
	token?: string;
	remember?: boolean;
	privateKey?: string;
}

export interface PrivateKeyLogin {
	privateKey: string;
	passphrase?: string;
	remember?: boolean;
}

interface PrivateKeyRequest {
	fingerprint: string;
	signature: string;
	remember: boolean;
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
	return new Promise(subscribe);
}

/**
 * Clear Authenticated object
 */
export function clear() {
	_set(null);
}

/**
 * Setup the authenticated object after successful authentication event
 */
export function setupAuthenticated(body: AuthenticatedJwt) {
	const { authenticated, jwt } = body;

	const expires = local(authenticated.session.expires_at);

	setJwt(jwt, expires);
	setCsrf(authenticated.session.csrf, expires);
	set(authenticated);
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
	const response = await Api.post<Credentials, AuthenticatedJwt>(
		'/api/auth/login',
		undefined,
		credentials
	);

	if (!response.body) {
		throw new Error('No authenticated object found after login');
	}

	setupAuthenticated(response.body);

	const { authenticated } = response.body;

	if (authenticated.user.encrypted_private_key) {
		credentials.privateKey = await crypto.rsa.decryptPrivateKey(
			authenticated.user.encrypted_private_key,
			credentials.password
		);
	}

	if (!credentials.privateKey) {
		throw new Error('No private key found, please provide your private key when authenticating');
	}

	const fingerprint = await crypto.rsa.getFingerprint(credentials.privateKey);
	if (fingerprint !== authenticated.user.fingerprint) {
		throw new Error('Private key does not match user');
	}

	await crypto.set(await crypto.rsa.inputToKeyPair(credentials.privateKey));

	return authenticated;
}

/**
 * Takes the given private key and passphrase, tries to decrypt it and then perform authentication
 * @throws
 */
export async function loginWithPrivateKey(input: PrivateKeyLogin): Promise<Authenticated> {
	const { privateKey, passphrase } = input;

	let pk = privateKey;

	if (passphrase) {
		pk = await crypto.rsa.decryptPrivateKey(privateKey, passphrase);
	}

	return _loginWithPrivateKey(await crypto.rsa.inputToKeyPair(pk || ''), false);
}

/**
 * Attempt to decrypt the private key and get the current user from backend
 * @throws
 */
export async function loginWithPin(pin: string): Promise<Authenticated> {
	const pk = crypto.getAndDecryptPrivateKey(pin);

	return _loginWithPrivateKey(await crypto.rsa.inputToKeyPair(pk), false);
}

/**
 * Perform authentication with KeyPair object, performs fingerprint calculation and signature creation
 * @throws
 */
export async function _loginWithPrivateKey(
	kp: crypto.rsa.KeyPair,
	remember: boolean
): Promise<Authenticated> {
	const fingerprint = await crypto.rsa.getFingerprint(kp.input as string);
	const nonce = crypto.createFingerprintNonce(fingerprint);
	const signature = await crypto.rsa.sign(kp, nonce);

	const response = await Api.post<PrivateKeyRequest, AuthenticatedJwt>(
		'/api/auth/signature',
		{},
		{
			fingerprint,
			signature,
			remember
		}
	);

	if (!response.body) {
		throw new Error('No authenticated object found after private key login');
	}

	setupAuthenticated(response.body);

	await crypto.set(kp);

	return response.body.authenticated;
}
