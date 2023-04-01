import CryptoJS from 'crypto-js';
import { writable } from 'svelte/store';
import RSA from 'node-rsa';

const encryptedMnemonicStorageName = 'encrypted-secret';

const RSAb = 2048;
const environment = 'browser';
const privateKeyFormat = 'pkcs8-private-pem';
const publicKeyFormat = 'pkcs8-public-pem';
const signingScheme = 'pss-sha256';
const encryptionScheme = 'pkcs1_oaep';

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
 * Get the encrypted secret from the localStorage
 */
export function getEncryptedSecret(): string | null {
	return localStorage.getItem(encryptedMnemonicStorageName);
}
/**
 * Lets us know if we should even attempt the decryption
 */
export function hasEncryptedSecretKey(): boolean {
	return !!getEncryptedSecret();
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
	const key: Raw = new RSA({ b: RSAb }).importKey(publicKey, publicKeyFormat);
	key.setOptions({ environment, signingScheme, encryptionScheme });

	return { key, publicKey };
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
 * Take the given secret, encrypt it with a pin and store it in localStorage
 */
export function encryptSecretAndStore(secret: string, pin: string) {
	const encryptedMnemonic = encryptSecret(secret, pin);
	localStorage.setItem(encryptedMnemonicStorageName, encryptedMnemonic);
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
 * Encrypt the given input with the given pin
 */
export function encryptSecret(secret: string, pin: string): string {
	const encrypted = CryptoJS.AES.encrypt(secret, pin, { format: CryptoJS.format.OpenSSL });
	return encrypted.toString(CryptoJS.format.OpenSSL);
}

/**
 * Decrypt the given encrypted input with the given pin
 * @throws
 */
export function decryptSecret(encrypted: string, pin: string): string {
	const wordArray = CryptoJS.AES.decrypt(encrypted, pin);
	const value = wordArray.toString(CryptoJS.enc.Utf8);
	return value;
}

/**
 * Encrypt a message with given public key
 */
export async function encryptMessage(
	data: EncryptionData,
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

	key.setOptions({ encryptionScheme: 'pkcs1' });

	return key.encrypt(data.message, data.encoding || 'base64');
}

/**
 * Encrypt a message with given public key (pkcs1_oaep)
 */
export async function encryptOaepMessage(
	data: EncryptionData,
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

	key.setOptions({ encryptionScheme: 'pkcs1_oaep' });

	return key.encrypt(data.message, data.encoding || 'base64');
}

/**
 * Decrypt a message with stored private key
 */
export async function decryptMessage(
	message: string,
	encoding: Encoding = 'utf8'
): Promise<string> {
	const { key } = await _get();

	if (!key || !key.isPrivate()) {
		throw new Error('No privateKey, cannot decrypt message');
	}

	key.setOptions({ encryptionScheme: 'pkcs1' });

	return key.decrypt(message, encoding);
}

/**
 * Decrypt a message with stored private key (pkcs1_oaep)
 */
export async function decryptOaepMessage(
	message: string,
	encoding: Encoding = 'utf8'
): Promise<string> {
	const { key } = await _get();

	if (!key || !key.isPrivate()) {
		throw new Error('No privateKey, cannot decrypt message');
	}

	key.setOptions({ encryptionScheme: 'pkcs1_oaep' });

	return key.decrypt(message, encoding);
}
