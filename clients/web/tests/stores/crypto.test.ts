import { describe, it, expect, assert } from 'vitest';
import * as crypto from '../../src/lib/stores/crypto';

const privatePem =
	'-----BEGIN PRIVATE KEY-----' +
	'MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQDFzqMJ5KNQwDKy' +
	'YocJ8aolIgiAaft4JNgOfb837/V/o/XJYyIs7yXV9X3VL2wA7DDODwk3iN8ag6GM' +
	'9msEp7inqg9qfZzHIee9555t2WNfuJdH1C+99QIWLMHXlPvFqDtPHOj3j//MfAMU' +
	'3rs88fThCdgU5iw3fhMw/g4D8PTuwWR6a3tqqqecpzlSFvfWCK/5HJldAQralas6' +
	'+jkYqqqERwqli62s0kT2OS64Qq7jo0FOB93pp9HPHe4A1grb+eIoC4R5LVlnPCRk' +
	'9PMV536AhTbEYekhwSoSTSRiKXLKQ1kPsrCO1QZO1NxE49UGB42sc0k/TB+mWL28' +
	'18WJPt2lAgMBAAECggEAIvXdjP8S+k+t5idR1KkYuE1mkUOqBVcFtLH23O0VR8Tz' +
	'yO8zeBugZUtpPQePoC4ehhzUNTOEswv2vpJC4eS+1ytQZDLlRbCxY7gPIT0duipG' +
	'2pQfCATIpKCuderIAOw150qlxjN2M27roIGpOCFPdYKm5TK1N+2ZeLw+P+YTdCr8' +
	'XipNlLk5msTv132zqDPhQmnwtw5ThQ96hsRZrMcEbidsJWdiJahYWhrCuj5NbEPl' +
	'sqLQexceuoiRehJkC8h3vCCxCkJxdJuh+bIHiN/Tk2TG/KOMoPGSUger1kAS8OxJ' +
	'y8A4q8QK2h+h+U9WQyBfcQVhQNDd8uIS7KXh9RC9AQKBgQDwbUCBZIqeNYCj/1j8' +
	'Erc3K+Gq/UPE7ygOGJZatiIJQnJcNAS9cDwWKcF4DHBvtz3B3nk80pGNnDJMiNSP' +
	'612tbF6EIK+NGhshNMrCS9WvxHamo5oGM9qkAyhAE57d9k3sbMpfiZrSFCLr1JyX' +
	'0Qu8fx3BoZCn6HFTFUY3uih2wQKBgQDSnqsalCFxp7gKeRkapALH5VxHfJpRcrBC' +
	'Loxv4BOCnvJyAAZif/DR/DQ96Max9I+2UuFfyGSTVFTEQ6aNeqTWD1Hq/zD1O1uG' +
	'IaTkaL/S3Nj1dBz4guztXqSTBQ5pZhwhSr6rD8aqpQk8xybevEvVY8YQVK9F4v1S' +
	'xOa9xSBj5QKBgEHgXY1WpBinZkEJRTOEWUk3r9SvInOCaAI8wG3Ie9j3qOgUpLvX' +
	'Vc9oz4b6OZCSr8xADg4ZUCJyCuInl757aiaLi/Y+EnviDE7z7R6BsuI/PZd5Okm6' +
	'yYypBM1R0vTUeRNv15+Hz7ECLXNaxTFf6QxT9C5K+5zWNr7iFGROkKnBAoGBAKkD' +
	'uMzAWEIrU+3blcCiIrUkojOfkvqPLVA+qGXSi/WC9Y1z5au/fZIUcBvKI0CEv5qQ' +
	'0diaJ9NulgNVQl9ALuy0KImKtU/ljSGK+BZu1Jgyr0vxHJpz/grRqwFryk/cJ/Cz' +
	'WWROaZ9ghpQmQFP3CGe6BCPwwSI08BIuffeFK+PdAoGAbZE4yvCdiP/LPP5By9w9' +
	'e4oeigb96PbCjaH/81HaegUd1A2vWEdbgvIBx/ccdQgQM05EGbM0T0VaG0TCjMaZ' +
	'anp/AJ8u1eIDracvCmRatY2Z2Wk03ZPxyotzskk2kfLI6VDzGItu1geRqZR6w7qu' +
	'I4AaX1PngjIdZmeTgirqEb4=' +
	'-----END PRIVATE KEY-----';

describe('Crypto test', () => {
	// it('can encrypt and decrypt string with provided pin using AES', () => {
	// 	const secret = 'little secret';
	// 	const pin = '123456';
	// 	const encrypted = crypto.encryptSecret(secret, pin);

	// 	assert(encrypted !== secret, 'Encrypted value is the same as provided value');

	// 	const decrypted = crypto.decryptSecret(encrypted, pin);

	// 	expect(decrypted).toBe(secret);
	// });

	// it('can generate secret key from input', async () => {
	// 	const keypair = crypto.generateKey();

	// 	const { input } = crypto.inputToKeypair(keypair.input as string);

	// 	expect(input).toBe(keypair.input);
	// });

	// it('can sign messages and verify signatures', async () => {
	// 	const keypair = crypto.generateKey();
	// 	crypto.set(keypair);

	// 	const message = 'hello world';
	// 	const { signature, publicKey } = await crypto.sign(message);

	// 	expect(publicKey).toBe(keypair.publicKey);
	// 	expect(await crypto.verify(signature, message)).toBe(true);
	// });

	// it('can sign messages and verify signatures with specific input', async () => {
	// 	const keypair = crypto.inputToKeypair(privatePem);
	// 	crypto.set(keypair);

	// 	const keypair2 = crypto.inputToKeypair(privatePem);

	// 	expect(keypair2.publicKey).toBe(keypair.publicKey);

	// 	const message = '28004708';

	// 	const { signature, publicKey } = await crypto.sign(message);

	// 	expect(publicKey).toBe(keypair.publicKey);
	// 	expect(await crypto.verify(signature, message)).toBe(true);

	// 	console.log('PRIVATE KEY:');
	// 	console.log(keypair.input);
	// 	console.log('PUBLIC KEY:');
	// 	console.log(publicKey);
	// 	console.log('SIGNATURE:');
	// 	console.log(signature);
	// });

	// it('can encrypt and decrypt message with generated keys', async () => {
	// 	const keypair = crypto.inputToKeypair(privatePem);
	// 	crypto.set(keypair);

	// 	const message = 'hello world';

	// 	const encrypted = await crypto.encryptMessage({ message }, keypair.publicKey as string);
	// 	const decrypted = await crypto.decryptMessage(encrypted);

	// 	console.log('ENCRYPTED:', encrypted);

	// 	expect(message !== encrypted).toBe(true);
	// 	expect(message).toEqual(decrypted);
	// });

	// it('can encrypt and decrypt message with generated keys oaep', async () => {
	// 	const keypair = crypto.inputToKeypair(privatePem);
	// 	crypto.set(keypair);

	// 	const message = 'hello world';

	// 	const encrypted = await crypto.encryptOaepMessage({ message }, keypair.publicKey as string);
	// 	const decrypted = await crypto.decryptOaepMessage(encrypted);

	// 	console.log('ENCRYPTED OAEP:', encrypted);

	// 	expect(message !== encrypted).toBe(true);
	// 	expect(message).toEqual(decrypted);
	// });

	it('can decrypt message from rust backend', async () => {
		const keypair = crypto.inputToKeypair(privatePem);
		crypto.set(keypair);

		const message = 'hello world';
		const encrypted =
			'C7iduHKi0ta8McbTKUbIDDb9ka4mzpll5yyqd+u1qeeIf0SfQaVySAhnx4+B4hjlPav2XX+nD0yWv4wBm65h+/nUFw0jdwKA5wtducS4H/G2AJt2PfNCtRMjeiv2UIl1iXoO7ae0EyZ/e/ZvhyxZ5XtL7LnS0lN8cVgCDj4jDennC12OoSX7wvSI5TyFd5BbYi1dYkUMFqF9ZNxTJBYvWj7AhFMyak1dwHJ7p0ymq59jdszre3r9+ZoFjg+M/ji+CkuK1RdPIB/ppBVZGtGZeZvsf6IZ+YEyzehhb4V44TtD0uPvXM+w+46CS9gEUU3bQzSBfbIGyKpFAI7Zur/CNA==';

		const decrypted = await crypto.decryptMessage(encrypted);

		expect(message).toEqual(decrypted);
	});

	// it('can decrypt oaep message from rust backend', async () => {
	// 	const keypair = crypto.inputToKeypair(privatePem);
	// 	crypto.set(keypair);

	// 	const message = 'hello world';
	// 	const encrypted =
	// 		'tDR8nc5woi3iwxw4ILpTPttlvZp3Z7mjRcSlnb0YnxtMtxzg5cC4MB5ygDKXBT379NDISTc0Xmh99fu3+UmWDBsOQAoSHb4lFiNHxBGQ8ztAYi7uo1FLsiLT9zDuTbQjnPrezhTIOH85OilJknu8XxXWof4oZUui2akLIyTL4Q9D1cgpyopKkw7c3izZd76r7K8uG8KB/c9BngVuWkgPguh2mcyIUioOmXQ+HHBc7HhEcp5wnpnHoURVKNfKmguDjtVEU2+y3AyxQ8eGk25k6rjoLFzIGiZZdy/t73xNnVmEVtDP12jd+09DlEEoNlbsJMHro8SvAKsBR72PC5WIA==';

	// 	const decrypted = await crypto.decryptOaepMessage(encrypted);

	// 	expect(message).toEqual(decrypted);
	// });
});
