import { writable } from 'svelte/store';
import * as PGP from 'openpgp';
import { encryptSecret, decryptSecret } from './aes';

const encryptedMnemonicStorageName = 'encrypted-secret';

const RSAb = 2048;
const keyType = 'rsa';

export type Raw = PGP.Key | PGP.PrivateKey | PGP.PublicKey;

export interface EncryptionData {
	message: string | Buffer | Uint8Array;
	encoding?: 'utf8' | 'utf16' | 'utf16le' | 'utf16be' | 'hex' | 'base64' | 'buffer';
}

export interface Keypair {
	/**
	 * The key
	 */
	key: Raw | null;

	/**
	 * Private key armour string
	 */
	input: string | null;

	/**
	 * Public key armour string
	 */
	publicKey: string | null;

	/**
	 * Id of the key
	 */
	keyId: string | null;
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
	let keyId = null;

	if (key) {
		return keypairFromRaw({ key, publicKey });
	}

	if (publicKey) {
		const { key: raw } = await publicToRaw(publicKey);

		if (raw) {
			keyId = raw?.getKeyID().toHex();
		}
	}

	return { input: null, key: null, publicKey, keyId };
}

/**
 * Set the external keypair value into the internal one
 */
export async function set(keypair: Keypair) {
	if (keypair.input) {
		const raw = await privateToRaw(keypair.input);

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
 * Generate a random input in a format of Keypair
 */
export async function generateKey(userId?: PGP.UserID | string): Promise<Keypair> {
	if (typeof userId === 'string') {
		userId = { name: userId, email: userId };
	}

	const { privateKey } = await PGP.generateKey({
		userIDs: userId || [{ name: 'anonymous', email: '' }],
		type: keyType,
		rsaBits: RSAb,
		format: 'object'
	});

	return privateToKeypair(privateKey.armor());
}

/**
 * Generate a Keypair from private
 */
export async function privateToKeypair(input: string, passphrase?: string): Promise<Keypair> {
	const { key, publicKey } = await privateToRaw(input, passphrase);

	const keyId = key?.getKeyID().toHex() || null;

	return {
		input,
		key,
		publicKey,
		keyId
	};
}

/**
 * Generate a Keypair from public
 */
export async function publicToKeypair(input: string): Promise<Keypair> {
	const { key, publicKey } = await publicToRaw(input);

	const keyId = key?.getKeyID().toHex() || null;

	return {
		input,
		key,
		publicKey,
		keyId
	};
}

/**
 * Convert input to raw
 */
export async function privateToRaw(input: string, passphrase?: string): Promise<RawKeypair> {
	let key = await PGP.readPrivateKey({ armoredKey: input });

	if (passphrase) {
		key = await PGP.decryptKey({
			privateKey: key,
			passphrase
		});
	}

	if (!key.isPrivate()) {
		throw new Error('Input is not a privateKey');
	}

	return { key, publicKey: key.toPublic().armor() };
}

/**
 * Generate RawKeypair from public key, this can only be used to verify signatures
 */
export async function publicToRaw(input: string): Promise<RawKeypair> {
	const key: Raw = await PGP.readKey({ armoredKey: input });

	if (key.isPrivate()) {
		throw new Error('Input is not a publicKey');
	}

	return { key, publicKey: key.armor() };
}

/**
 * Generate a Keypair from input
 */
export function keypairFromRaw(internal: RawKeypair): Keypair {
	const { key, publicKey } = internal;

	const input = key?.isPrivate() ? key.armor() : null;

	const keyId = key?.getKeyID().toHex() || null;

	return {
		input,
		key,
		publicKey,
		keyId
	};
}

/**
 * Sign the given message with current secret key and return an object with signature and publicKey
 */
export async function sign(text: string): Promise<{ signature: string; publicKey: string }> {
	const { key, publicKey } = await _get();

	if (!key || !key.isPrivate()) {
		throw new Error('No privateKey, cannot sign message');
	}

	const unsignedMessage = await PGP.createCleartextMessage({ text });
	const signature = await PGP.sign({
		message: unsignedMessage,
		signingKeys: key
	});

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

	const { key } = await publicToRaw(publicKey);

	if (!key) {
		throw new Error('No publicKey, cannot verify message');
	}

	const signedMessage = await PGP.readCleartextMessage({
		cleartextMessage: signature
	});

	const verificationResult = await PGP.verify({
		// @ts-ignore - there is a bug in openpgp typings
		message: signedMessage,
		verificationKeys: key
	});

	const { verified } = verificationResult.signatures[0];

	try {
		await verified;

		return verificationResult.data === message;
	} catch (e) {
		return false;
	}
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
export async function decryptSecretAndSet(encryptedMnemonic: string, pin: string) {
	const input = decryptSecret(encryptedMnemonic, pin);
	_set(await privateToRaw(input));
}

/**
 * Encrypt a message with given public key
 */
export async function encryptMessage(text: string, inputPublicKey?: string): Promise<string> {
	let { key } = await _get();

	if (inputPublicKey) {
		const { key: publicRawKey } = await publicToRaw(inputPublicKey as string);

		key = publicRawKey;
	}

	if (!key) {
		throw new Error('No publicKey, cannot encrypt message');
	}

	return await PGP.encrypt({
		message: await PGP.createMessage({ text }),
		encryptionKeys: key
	});
}

/**
 * Decrypt a message with stored private key
 */
export async function decryptMessage(armoredMessage: string): Promise<string> {
	const { key } = await _get();

	if (!key || !key.isPrivate()) {
		throw new Error('No privateKey, cannot decrypt message');
	}

	const message = await PGP.readMessage({
		armoredMessage
	});
	const { data } = await PGP.decrypt({
		message,
		decryptionKeys: key
	});

	return data;
}
