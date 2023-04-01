import CryptoJS from 'crypto-js';
import elliptic from 'elliptic';
import bip39 from 'bip39';
import { writable } from 'svelte/store';

const encryptedSecretStorageName = 'encrypted-secret';

export interface Keypair {
	publicKey: string | null;
	secretKey: string | null;
}

export const { subscribe, set: _set } = writable<Keypair>({
	publicKey: null,
	secretKey: null
});

/**
 * Encrypt the given input with the given pin
 */
export function encrypt(value: string, pin: string): string {
	const encrypted = CryptoJS.AES.encrypt(value, pin, { format: CryptoJS.format.OpenSSL });
	return encrypted.toString(CryptoJS.format.OpenSSL);
}

/**
 * Decrypt the given encrypted input with the given pin
 * @throws
 */
export function decrypt(encrypted: string, pin: string): string {
	const decryptedBytes = CryptoJS.AES.decrypt(encrypted, pin);
	const decryptedValue = decryptedBytes.toString(CryptoJS.enc.Utf8);
	return decryptedValue;
}

/**
 * Take the given secret, encrypt it with a pin and store it in localStorage
 */
export function encryptAndStore(secret: string, pin: string) {
	const encryptedSecret = encrypt(secret, pin);
	localStorage.setItem(encryptedSecretStorageName, encryptedSecret);
}

/**
 * Try to get the encrypted secret from the localStorage and decrypt it with the given pin
 * if decryption is okay then set the keypair with new values.
 *
 * @throws
 */
export function decryptAndSet(encryptedSecret: string, pin: string) {
	const secret = decrypt(encryptedSecret, pin);
	set(generateKeyFrom(secret));
}

/**
 * Get the encrypted secret from the localStorage
 */
export function getEncryptedSecret(): string | null {
	return localStorage.getItem(encryptedSecretStorageName);
}
/**
 * Lets us know if we should even attempt the decryption
 */
export function hasEncryptedSecretKey(): boolean {
	return !!getEncryptedSecret();
}

/**
 * Convert seed mnemonic to raw secret key
 */
export function mnemonicToKeypairRaw(mnemonic: string): elliptic.ec.KeyPair {
	const ec = new elliptic.ec('secp256k1');
	const keyPair = ec.keyFromPrivate(bip39.mnemonicToSeedSync(mnemonic));

	return keyPair;
}

/**
 * Convert seed mnemonic to keypair - alias for generateKeyFrom
 */
export function mnemonicToKeypair(mnemonic: string): Keypair {
	return generateKeyFrom(mnemonic);
}

/**
 * Convert hex to mnemonic, this will work only for hex secret key
 */
export function hexToMnemonic(hex: string): string {
	const mnemonic = bip39.entropyToMnemonic(Buffer.from(hex, 'hex'));
	return mnemonic;
}

/**
 * Generate a random keypair in a format of Keypair
 */
export function generateKey(): Keypair {
	return generateKeyFrom(bip39.generateMnemonic(256));
}

/**
 * Generate a key from mnemonic
 */
export function generateKeyFrom(mnemonic: string): Keypair {
	const ec = new elliptic.ec('secp256k1');

	const hex = bip39.mnemonicToSeedSync(mnemonic);

	const keyPair = ec.keyFromPrivate(hex);

	return {
		publicKey: keyPair.getPublic().encode('hex', true),
		secretKey: mnemonic
	};
}

/**
 * Get the value right away
 */
export async function _get(): Promise<Keypair> {
	return new Promise((resolve) => subscribe(resolve));
}

/**
 * Gets the authenticated object if it exists
 */
export async function get(): Promise<Keypair | null> {
	const keypair = await _get();

	if (!keypair || !keypair.secretKey || !keypair.publicKey) {
		return null;
	}

	return keypair;
}

/**
 * Set the keypair value
 */
export function set(glob: Keypair) {
	_set(glob);
}

/**
 * Clear the keypair value
 */
export function clearKeypair() {
	_set({
		publicKey: null,
		secretKey: null
	});
}

/**
 * Sign the given message with current secret key and return an object with signature and pubkey
 */
export async function sign(message: string): Promise<{ signature: string; pubkey: string }> {
	const { secretKey } = await _get();

	if (!secretKey) {
		throw new Error('No secretKey on keypair, cannot sign message');
	}

	const keypair = mnemonicToKeypairRaw(secretKey);

	// TODO: Maybe add hashing here... The signature is not matching for some reason!
	const signature = keypair.sign(CryptoJS.SHA256(message).toString());

	return {
		signature: signature.toDER('hex'),
		pubkey: keypair.getPublic().encode('hex', true)
	};
}

/**
 * Verify signed message
 */
export async function verify(signature: string, message: string): Promise<boolean> {
	const { secretKey } = await _get();

	if (!secretKey) {
		throw new Error('No secretKey on keypair, cannot verify message');
	}

	const keypair = mnemonicToKeypairRaw(secretKey);

	return keypair.verify(CryptoJS.SHA256(message).toString(), signature);
}
