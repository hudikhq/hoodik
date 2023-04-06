import CryptoJS from 'crypto-js';

/**
 * Encrypt the given input with the given pin
 */
export function encrypt(secret: string, pin: string): string {
	const encrypted = CryptoJS.AES.encrypt(secret, pin, {
		format: CryptoJS.format.OpenSSL,
		salt: 128
	});
	return encrypted.toString(CryptoJS.format.OpenSSL);
}

/**
 * Decrypt the given encrypted input with the given pin
 * @throws
 */
export function decrypt(encrypted: string, pin: string): string {
	const wordArray = CryptoJS.AES.decrypt(encrypted, pin, {
		format: CryptoJS.format.OpenSSL,
		salt: 128
	});
	return wordArray.toString(CryptoJS.enc.Utf8);
}
