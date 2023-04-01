import CryptoJS from 'crypto-js';
import bip39 from 'bip39';
import { writable } from 'svelte/store';
import RSA from 'node-rsa';
import HDNode from 'hdkey';

export interface Raw extends HDNode {
	mnemonic?: string;
}

const encryptedMnemonicStorageName = 'encrypted-secret';

export interface Keypair {
	mnemonic: string | null;
	key: Raw | null;
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

	return { mnemonic: null, key: null, publicKey };
}

/**
 * Set the external keypair value into the internal one
 */
export function set(keypair: Keypair) {
	if (keypair.mnemonic) {
		const raw = mnemonicToRaw(keypair.mnemonic);

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
 * Convert mnemonic to raw
 */
export function mnemonicToRaw(mnemonic: string): RawKeypair {
	const seed = bip39.mnemonicToSeedSync(mnemonic);
	const key: Raw = HDNode.fromMasterSeed(seed);

	key.mnemonic = mnemonic;

	return { key, publicKey: key.publicExtendedKey };
}

/**
 * Generate RawKeypair from public key, this can only be used to verify signatures
 */
export function publicToRaw(publicKey: string): RawKeypair {
	const key = HDNode.fromExtendedKey(publicKey);

	return { key, publicKey: key.publicExtendedKey };
}

/**
 * Generate a random mnemonic in a format of Keypair
 */
export function generateKey(): Keypair {
	return mnemonicToKeypair(bip39.generateMnemonic(24));
}

/**
 * Generate a Keypair from mnemonic
 */
export function mnemonicToKeypair(mnemonic: string): Keypair {
	const { key, publicKey } = mnemonicToRaw(mnemonic);

	return {
		mnemonic,
		key,
		publicKey
	};
}

/**
 * Generate a Keypair from mnemonic
 */
export function keypairFromRaw(internal: RawKeypair): Keypair {
	const { key, publicKey } = internal;

	return {
		mnemonic: key?.mnemonic || null,
		key,
		publicKey
	};
}

/**
 * Sign the given message with current secret key and return an object with signature and publicKey
 */
export async function sign(message: string): Promise<{ signature: string; publicKey: string }> {
	const { key, publicKey } = await _get();

	if (!key) {
		throw new Error('No privateKey, cannot sign message');
	}

	const messageHash = Buffer.from(CryptoJS.SHA256(message).toString(), 'hex');

	const signature = key.sign(messageHash);

	return {
		signature: signature.toString('hex'),
		publicKey: publicKey || key.publicExtendedKey
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

	const messageHash = Buffer.from(CryptoJS.SHA256(message).toString(), 'hex');

	return !!publicToRaw(publicKey).key?.verify(messageHash, Buffer.from(signature, 'hex'));
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
	const mnemonic = decryptSecret(encryptedMnemonic, pin);
	_set(mnemonicToRaw(mnemonic));
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
export async function encryptMessage(message: string, publicKey?: string): Promise<string> {
	if (!publicKey) {
		const { publicKey: _publicKey } = await _get();
		publicKey = _publicKey as string;
	}

	if (!publicKey) {
		throw new Error('No publicKey, cannot encrypt message');
	}

	const key = new RSA(publicKey, 'pkcs1-public-der');
	return key.encrypt(message, 'base64');
}

/**
 * Decrypt a message with stored private key
 */
export async function decryptMessage(message: string): Promise<string> {
	const { key } = await _get();

	if (!key) {
		throw new Error('No privateKey, cannot decrypt message');
	}

	const privateKey = key.privateExtendedKey;
	const rsa = new RSA(privateKey, 'pkcs1-private');
	return rsa.decrypt(message, 'utf8');
}
