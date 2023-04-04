import { writable } from 'svelte/store';
import RSA from 'node-rsa';
import { decrypt as decryptSecret } from './aes';
import constants from 'constants';
import crypto from 'crypto';

const RSAb = 1024;
const environment = 'node';
const privateKeyFormat = 'pkcs1';
const publicKeyFormat = 'pkcs1-public-pem';
const signingScheme = 'pss-sha256';

// const encryptionScheme = 'pkcs1';
const encryptionScheme: RSA.AdvancedEncryptionSchemePKCS1 = {
	scheme: 'pkcs1',
	padding: constants.RSA_PKCS1_PADDING
};

export interface Raw extends RSA {
	input?: string;
}

export type Encoding = RSA.Encoding;
export type Data = RSA.Data;

export interface EncryptionData {
	message: Data;
	encoding?: Encoding;
}

export interface Keypair {
	/**
	 * Private RSA key string
	 */
	input: string | null;

	/**
	 * The RSA key
	 */
	key: Raw | null;

	/**
	 * Public RSA key string
	 */
	publicKey: string | null;
}

interface RawKeypair {
	key: Raw | null;
	publicKey: string | null;
}

export const { subscribe, set: _set } = writable<RawKeypair>({ key: null, publicKey: null });

/**
 * Get the raw keypair value
 */
export async function _get(): Promise<RawKeypair> {
	return new Promise((resolve) => subscribe((value) => resolve(value)));
}

/**
 * Get the keypair value that is presented outside
 */
export async function get(): Promise<Keypair> {
	const { key, publicKey } = await _get();

	if (key) {
		return keypairFromRaw({ key, publicKey });
	}

	return { input: null, key: null, publicKey };
}

/**
 * Set the external keypair value into the internal one
 */
export function set(keypair: Keypair) {
	if (keypair.input) {
		const raw = inputToRaw(keypair.input);

		_set(raw);
	} else if (keypair.publicKey) {
		_set({
			key: null,
			publicKey: keypair.publicKey
		});
	} else {
		clear();
	}
}

/**
 * Clear the keypair value
 */
export function clear() {
	_set({ key: null, publicKey: null });
}

/**
 * Convert input to raw
 */
export function inputToRaw(input: string): RawKeypair {
	const key: Raw = new RSA({ b: RSAb }).importKey(input, privateKeyFormat);
	key.setOptions({ environment, signingScheme, encryptionScheme });

	key.input = input;

	return { key, publicKey: key.exportKey(publicKeyFormat) };
}

/**
 * Generate RawKeypair from public key, this can only be used to verify signatures
 */
export function publicToRaw(publicKey: string): RawKeypair {
	const key = new RSA(publicKey, publicKeyFormat, {
		environment,
		signingScheme,
		encryptionScheme
	});

	return { key, publicKey };
}

/**
 * Generate key id from string
 *
 * @throws
 */
export async function getFingerprint(input: string): Promise<string> {
	try {
		const { key } = publicToRaw(input);

		if (!key) {
			throw new Error('Not public key, or not a public key');
		}

		return getFingerprintFromRaw(key);
	} catch (e) {
		const { publicKey } = inputToRaw(input);

		if (!publicKey) {
			throw new Error(`Not a public key or a private key, upstream error: ${e}`);
		}

		const { key } = publicToRaw(publicKey);

		if (!key) {
			throw new Error(`Not a public key or a private key, upstream error: ${e}`);
		}

		return getFingerprintFromRaw(key);
	}
}

/**
 * Generate a key id from given raw key
 */
export async function getFingerprintFromRaw(key: Raw): Promise<string> {
	const { n } = key.exportKey('components-public');

	const newN = Array.prototype.map.call(n, (byte) => byte as number) as number[];
	newN.shift();
	const buffer = Buffer.from(newN);

	const ab = await crypto.subtle.digest('SHA-256', buffer);

	return Buffer.from(ab).toString('hex');
}

/**
 * Generate a random input in a format of Keypair
 */
export function generateKey(): Keypair {
	return inputToKeypair(new RSA({ b: RSAb }).generateKeyPair().exportKey(privateKeyFormat));
}

/**
 * Generate a Keypair from input
 */
export function inputToKeypair(input: string): Keypair {
	const { key, publicKey } = inputToRaw(input);

	return {
		input,
		key,
		publicKey
	};
}

/**
 * Generate a Keypair from input
 */
export function keypairFromRaw(internal: RawKeypair): Keypair {
	const { key, publicKey } = internal;

	return {
		input: key?.input || null,
		key,
		publicKey
	};
}

/**
 * Sign the given message with current secret key and return an object with signature and publicKey
 */
export async function sign(message: string): Promise<{ signature: string; publicKey: string }> {
	const { key, publicKey } = await _get();

	if (!key || !key.isPrivate()) {
		throw new Error('No privateKey, cannot sign message');
	}

	const signature = key.sign(message, 'hex');

	return {
		signature,
		publicKey: publicKey as string
	};
}

/**
 * Verify the message with the given public key or the stored one
 */
export async function verify(
	signature: string,
	message: string,
	publicKey?: string
): Promise<boolean> {
	const { publicKey: _publicKey } = await _get();

	if (!publicKey) {
		publicKey = _publicKey as string;
	}

	if (!publicKey) {
		throw new Error('No publicKey, cannot verify message');
	}

	const { key } = publicToRaw(publicKey);

	if (!key) {
		throw new Error('No publicKey, cannot verify message');
	}

	return key.verify(message, Buffer.from(signature, 'hex'));
}

/**
 * Try to get the encrypted secret from the localStorage and decrypt it with the given pin
 * if decryption is okay then set the keypair with new values.
 *
 * @throws
 */
export function decryptSecretAndSet(encryptedMnemonic: string, pin: string) {
	const input = decryptSecret(encryptedMnemonic, pin);
	_set(inputToRaw(input));
}

/**
 * Encrypt a message with given public key
 */
export async function encryptMessage(message: string, inputPublicKey?: string): Promise<string> {
	let key;

	if (inputPublicKey) {
		const { key: p } = publicToRaw(inputPublicKey as string);
		key = p;
	} else {
		const { key: p } = await _get();
		key = p;
	}

	if (!key) {
		throw new Error('No publicKey, cannot encrypt message');
	}

	if (!key.isPublic()) {
		throw new Error('Key is not public, cannot encrypt message');
	}

	key.setOptions({
		encryptionScheme: {
			scheme: 'pkcs1',
			padding: constants.RSA_PKCS1_PADDING
		}
	});

	return key.encrypt(message, 'base64');
}

/**
 * Encrypt a message with given public key (pkcs1_oaep)
 */
export async function encryptOaepMessage(
	message: string,
	inputPublicKey?: string
): Promise<string> {
	let { key } = await _get();

	if (inputPublicKey) {
		const { key: publicRawKey } = publicToRaw(inputPublicKey as string);

		key = publicRawKey;
	}

	if (!key) {
		throw new Error('No publicKey, cannot encrypt message');
	}

	key.setOptions({
		encryptionScheme: {
			scheme: 'pkcs1_oaep',
			hash: 'sha256'
		}
	});

	return key.encrypt(Buffer.from(message, 'utf8')).toString('base64');
}

/**
 * Decrypt a message with stored private key
 */
export async function decryptMessage(message: string): Promise<string> {
	const { key } = await _get();

	if (!key || !key.isPrivate()) {
		throw new Error('No privateKey, cannot decrypt message');
	}

	key.setOptions({
		encryptionScheme: {
			scheme: 'pkcs1',
			padding: constants.RSA_PKCS1_PADDING
		}
	});

	return key.decrypt(Buffer.from(message, 'base64'), 'utf8');
}

/**
 * Decrypt a message with stored private key (pkcs1_oaep)
 */
export async function decryptOaepMessage(message: string): Promise<string> {
	const { key } = await _get();

	if (!key || !key.isPrivate()) {
		throw new Error('No privateKey, cannot decrypt message');
	}

	key.setOptions({
		encryptionScheme: {
			scheme: 'pkcs1_oaep',
			hash: 'sha256'
		}
	});

	return key.decrypt(Buffer.from(message, 'base64'), 'utf8');
}
