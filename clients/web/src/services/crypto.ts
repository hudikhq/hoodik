import CryptoJS from 'crypto-js';
import Crypto from 'crypto';
import elliptic from 'elliptic';
import bip39 from 'bip39';
import { writable } from 'svelte/store';
import { webcrypto } from 'node:crypto';

const encryptedSecretStorageName = 'encrypted-secret';

if (!globalThis.crypto) globalThis.crypto = webcrypto as Crypto;

export interface Globals {
	publicKey: string | null;
	secretKey: string | null;
}

export const { subscribe, set: _set } = writable<Globals>({
	publicKey: null,
	secretKey: null
});

/**
 * Encrypt the given input with the given pin
 */
export function encrypt(value: string, pin: string): string {
	const encrypted = CryptoJS.AES.encrypt(value, pin);
	return encrypted.toString();
}

/**
 * Decrypt the given input with the given pin
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
 * Convert seed mnemonic to raw secret key
 */
export function mnemonicToKeypair(mnemonic: string): elliptic.ec.KeyPair {
	const ec = new elliptic.ec('secp256k1');
	const keyPair = ec.keyFromPrivate(bip39.mnemonicToSeedSync(mnemonic));

	return keyPair;
}

/**
 * Convert hex to mnemonic, this will work only for hex secret key
 */
export function hexToMnemonic(hex: string): string {
	const mnemonic = bip39.entropyToMnemonic(Buffer.from(hex, 'hex'));
	return mnemonic;
}

/**
 * Generate a random keypair in a format of Globals
 */
export function generateKey(): Globals {
	return generateKeyFrom(bip39.generateMnemonic());
}

/**
 * Generate a key from mnemonic
 */
export function generateKeyFrom(mnemonic: string): Globals {
	const ec = new elliptic.ec('secp256k1');

	const hex = bip39.mnemonicToSeedSync(mnemonic);

	const keyPair = ec.keyFromPrivate(hex);

	return {
		publicKey: keyPair.getPublic().toString(),
		secretKey: mnemonic
	};
}

/**
 * Get the value right away
 */
export async function get(): Promise<Globals> {
	return new Promise((resolve) => subscribe(resolve));
}

/**
 * Set the globals value
 */
export function set(glob: Globals) {
	_set(glob);
}

/**
 * Clear the globals value
 */
export function clearGlob() {
	_set({
		publicKey: null,
		secretKey: null
	});
}

/**
 * Try to get the encrypted secret from the localStorage and decrypt it with the given pin
 * if decryption is okay then set the globals with new values.
 *
 * @throws
 */
export function decryptAndSet(encryptedSecret: string, pin: string) {
	const secret = decrypt(encryptedSecret, pin);
	set(generateKeyFrom(secret));
}

/**
 * Sign the given message with current secret key and return an object with signature and pubkey
 */
export async function sign(message: string): Promise<{ signature: string; pubkey: string }> {
	const { secretKey } = await get();

	if (!secretKey) {
		throw new Error('No secretKey on globals, cannot sign message');
	}

	const keypair = mnemonicToKeypair(secretKey);

	const signature = keypair.sign(message);

	return {
		signature: signature.toDER('hex'),
		pubkey: keypair.getPublic().toString()
	};
}

/**
 * Verify signed message
 */
export async function verify(signature: string, message: string): Promise<boolean> {
	const { secretKey } = await get();

	if (!secretKey) {
		throw new Error('No secretKey on globals, cannot verify message');
	}

	const keypair = mnemonicToKeypair(secretKey);

	return keypair.verify(message, signature);
}
