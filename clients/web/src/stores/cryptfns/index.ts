import { writable } from 'svelte/store';
import * as rsa from './rsa';
import * as aes from './aes';

const ENCRYPTED_PRIVATE_KEY_LOCAL_STORAGE = 'encrypted-secret';

export { rsa, aes };

export const { subscribe, set: _set } = writable<rsa.KeyPair>({
	key: null,
	publicKey: null,
	input: null,
	fingerprint: null
});

/**
 * Get the stored keypair value
 */
export async function get(): Promise<rsa.KeyPair> {
	return new Promise((resolve) => subscribe((value) => resolve(value)));
}

/**
 * Set the external keypair value into the internal one
 */
export async function set(keypair: rsa.KeyPair) {
	if (keypair.input) {
		const kp = await rsa.inputToKeyPair(keypair.input);
		_set(kp);
	} else if (keypair.publicKey) {
		const kp = await rsa.publicToKeyPair(keypair.publicKey);
		_set(kp);
	} else {
		clear();
	}
}

/**
 * Clear the keypair value
 */
export async function clear() {
	await _set({ key: null, publicKey: null, input: null, fingerprint: null });
}

/**
 * Get the encrypted private key from the localStorage
 */
export function getEncryptedPrivateKey(): string | null {
	return (localStorage || global.localStorage).getItem(ENCRYPTED_PRIVATE_KEY_LOCAL_STORAGE);
}
/**
 * Lets us know if we should even attempt the decryption
 */
export function hasEncryptedPrivateKey(): boolean {
	return !!getEncryptedPrivateKey();
}

/**
 * Take the given private key, encrypt it with a pin and store it in localStorage
 */
export function encryptPrivateKeyAndStore(pk: string, pin: string) {
	const encrypted = aes.encrypt(pk, pin);

	(localStorage || global.localStorage).setItem(ENCRYPTED_PRIVATE_KEY_LOCAL_STORAGE, encrypted);
}

/**
 * Remove the encrypted private key from storage
 */
export function clearEncryptedSecret() {
	if (hasEncryptedPrivateKey()) {
		(localStorage || global.localStorage).removeItem(ENCRYPTED_PRIVATE_KEY_LOCAL_STORAGE);
	}
}

/**
 * Get the encrypted private key from storage and decrypt it
 */
export function getAndDecryptPrivateKey(pin: string) {
	const pk = getEncryptedPrivateKey();

	if (!pk) {
		throw new Error('No encrypted private key found');
	}

	return aes.decrypt(pk, pin);
}

/**
 * Create a timed nonce for authentication via private key
 */
export function createFingerprintNonce(fingerprint: string): string {
	const timestamp = parseInt(`${Date.now() / 1000}`);
	const flat = `${parseInt(`${timestamp / 60}`)}`;

	return `${fingerprint}-${flat}`;
}
