import CryptoJS from 'crypto-js';

const encryptedSecretLocation = 'encrypted-secret';

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
 * Get the encrypted secret from the localStorage
 */
export function getEncryptedSecret(): string | null {
	return localStorage.getItem(encryptedSecretLocation);
}
/**
 * Lets us know if we should even attempt the decryption
 */
export function hasEncryptedSecretKey(): boolean {
	return !!getEncryptedSecret();
}

/**
 * Take the given secret, encrypt it with a pin and store it in localStorage
 */
export function encryptSecretAndStore(secret: string, pin: string) {
	const encrypted = encryptSecret(secret, pin);
	localStorage.setItem(encryptedSecretLocation, encrypted);
}
