import { describe, it, expect } from 'vitest';
import { pgp as crypto } from '../../src/lib/stores/cryptfns';

const privatePem = `-----BEGIN PGP PRIVATE KEY BLOCK-----

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
m7XWSSfSe2d1IZTu9ygZeZ35mP4dMxEtxRlmmQ==
=myk9
-----END PGP PRIVATE KEY BLOCK-----`;

const publicPem = `-----BEGIN PGP PUBLIC KEY BLOCK-----

xsBNBGQpO4sBCACvdk4IZcjwBh7h+MpVL2Hc3sdAacTkw7ThaeKFU0Zu+rFl
Bl4lemShlKHFARDRRB2POMwm8r8mGOaBsuLRtuJVygTOei1ZzDvRHtu2k9bj
9h7sdVvy/eMv+nx481oUiwaysiVCh+8VE0u2TT1x5EXp65bpkjD4bUt4vCFG
0d34XHlZC9/7vKBwhW8dgR9ZLtdGLbXmiSOCOELiUjUfc/988LwQa7PEL9EY
FMnZhr7KUuHubcNpv9Sj33rWPyBQ781fKrZQa0K9xJshV+j+/t3QX5b4zYEO
f7ta3a1R7IfexA5cTZ0vkf3qKaBDRmMgcg+yPN0lLPe45ksQPEgv8eXdABEB
AAHNCWFub255bW91c8LAigQQAQgAPgWCZCk7iwQLCQcICZBpGcY6FZt+5gMV
CAoEFgACAQIZAQKbAwIeARYhBLf3pJdg7c+r/0UkCmkZxjoVm37mAABNkAf+
IbOCLkT9B94X7JrKiSfedYfugFc9NuadIzqH52mZNWSsIGKosc5/y4wFg5nN
xvhZTmSvzv1vW5oaeu0NUJLKs0ZsPrhZS8ig+EmKNDWlRUBSLOkj9V5BndP4
yvnHeuMD+5irnin9qPxvk5y4+yAQA3QlzU1bHFWXiEgqoLT2YPEFKgVGtzTR
dxU4o94LZMdIobPxSUuEmYQ0E8Xjlswne7MmjQtBjO2Omhu+qJjRZ0jU3M8G
gbsmjRFegcAbA+bBE14etA7Vnen3IAtFVfNCm/ykll4szevtFAMCOfsQnnvb
EqDAfUjlVLMMEoiwIFqTv+Mso5CBRD07qiJqL50Opc7ATQRkKTuLAQgAtB/o
CrkLOWZIrjypU8qvZOp+pF9M1H3VdGjCmfIlehKpPw3PEdq85slHpXaE0PH9
9zbRCqbi2SH+pLUBgPq6+rhNlcW4X1wV90y3Vtp9zzmo6AdGgIHRZiDkHXOG
VaUNMpw+G3IWu3wP6iqK6t4oQDvZHlSYk4gNpJooV3T8DJOTvOmODPa90WIX
4J0jAAsXvYBNV6ehHPCuZfHTfKXomOqUZrGWjVtBF7hZpyqNiGoiIpgD+cac
ufohXCGgT6Zno+/Ocd5P7JLmcXH6fIsYpk3XhrxLbl7swMka5ILM7HMp+TMj
jxuZDJSgs/dwEeFuVbYoUiriuIoeJDuXy6RLgwARAQABwsB2BBgBCAAqBYJk
KTuLCZBpGcY6FZt+5gKbDBYhBLf3pJdg7c+r/0UkCmkZxjoVm37mAABjywgA
mWnAN3hBooEM3iEsmVgu+ZZCV/ZALbgQRbJW2YrKUUg4TcqzBqyHD3gxBsnf
HcsXCi4BTcSZXW+eRSCoupXj63EPMDDb9iTCP8vayoK3u/ISCxXg6B0N2Z3x
b2YnUanahP+CRLLzR6pxNHzjM1RdQVaO/Nkw0e3EydV81QqNHfZ96lPtza8m
WDA0y1ovlAsbQuL4GDL6/Hn8GYEK8fpo7hljETf4WQd0UG4GJJ6IIWP2LNSK
TeUWEsEIzJHqZAaeqnoTTJiPFcBQR3C8Uk7RUud+A5Aais/oWgYjmxwylAPd
Auy2m7XWSSfSe2d1IZTu9ygZeZ35mP4dMxEtxRlmmQ==
=0KN2
-----END PGP PUBLIC KEY BLOCK-----`;

describe('Crypto test', () => {
	it('GPG: can generate secret key from input', async () => {
		const keypair = await crypto.generateKey();

		const { input } = await crypto.privateToKeypair(keypair.input as string);

		expect(input).toBe(keypair.input);
		expect(input !== keypair.publicKey).toBe(true);
	});

	it('GPG: can sign messages and verify signatures', async () => {
		const keypair = await crypto.privateToKeypair(privatePem);
		await crypto.set(keypair);

		const message = 'hello world';
		const { signature, publicKey } = await crypto.sign(message);

		expect(publicKey).toBe(keypair.publicKey);
		expect(await crypto.verify(signature, message)).toBe(true);
	});

	it('GPG: can sign messages and verify signatures with specific input', async () => {
		const keypair = await crypto.privateToKeypair(privatePem);
		await crypto.set(keypair);

		const keypair2 = await crypto.privateToKeypair(privatePem);

		expect(keypair2.publicKey).toBe(keypair.publicKey);

		const message = '28004708';

		const { signature, publicKey } = await crypto.sign(message);

		expect(publicKey).toBe(keypair.publicKey);
		expect(await crypto.verify(signature, message)).toBe(true);

		console.log('PRIVATE KEY:');
		console.log(keypair.input);
		console.log('PUBLIC KEY:');
		console.log(publicKey);
		console.log('SIGNATURE:');
		console.log(signature);
		console.log('KEY ID:');
		console.log(keypair.keyId);
	});

	it('GPG: can encrypt and decrypt message with generated keys', async () => {
		const keypair = await crypto.privateToKeypair(privatePem);
		await crypto.set(keypair);

		const message = 'hello world';

		const encrypted = await crypto.encryptMessage(message, keypair.publicKey as string);
		const decrypted = await crypto.decryptMessage(encrypted);

		console.log('ENCRYPTED:');
		console.log(encrypted);

		expect(message !== encrypted).toBe(true);
		expect(message).toEqual(decrypted);
	});

	it('GPG: both private and public key return the same ID', async () => {
		const { keyId: priv1 } = await crypto.privateToKeypair(privatePem);
		const { keyId: priv2 } = await crypto.privateToKeypair(privatePem);
		const { keyId: pub } = await crypto.publicToKeypair(publicPem);

		expect(priv1).toEqual(priv2);
		expect(priv2).toEqual(pub);
	});
});
