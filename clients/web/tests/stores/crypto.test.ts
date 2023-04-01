import { describe, it, expect, assert } from 'vitest';
import * as crypto from '../../src/lib/stores/cryptoRSA';

const privateInputKey =
	'308204a302010002820101008010f3c56677bc61dd263edc275667b3233e1f6cb7bc013fc8a9edb4d3bb8682a1d48095229c5f7681d5673748402f843024db08da3675219dbe0d3ecc7f3d4673382672833f1b74d813e34709df5081284ab2c85ab26c91436050cef09aa26993521daae17b87f9423599df6fd6284a6ce02a2935f691dd81b39e83ad34a3f2ed585679d61408deca8cfa459776732a1a4627278cbad682d48c2708dc9d3af9cda2f7bc3bfbd78ea50a518adf92e50aee59f6f557f8ee29a410cdf52706a7822a243d07bbe821855cbc41b2694a32d4e003a9017d5a88310a51f25eef797575d4bd1256c3baf03f8c3c62ee4bbadade1f79ae98ca6059cbc89fef9bbf8b13610203010001028201007b71dbc877d10ae13a8ed720d73a4e933a717351147a40a9fefeed86d4617a730913112eb0421332e44b9446917a6d52fa254c8ce7ea7e557cfbe940935642f659b1d23da78d7925d3f2b7ad8e32982327777985ef06f5e7c00e6356564e7827543e5f228c5ecaded5975d4f273a43741a26a9fd376b09877eb269257c1bb5bbe1e63924ff2514e3c8e292b7b761398a2603654e54031cd26278e246946ef1bbc6ac291ad6b823b07511c5fcee7b6693f1490ccb8d551ffb68476b98f494e39c38a806a65f69d211422cc8b8116277c2a0a75a3b83859bba8478308731ad485161ea626135ae1f741a0d14ed0352a79b7b70ff1e914099dc09ebe36ac79e8f5102818100c5c8a6638eeb43a5ad582d773cb4dedf9cd37a62ee935990cccdd6c869eb6b1251c9a9d22231a524cc01d0f7f88e57fabe8b26366f034ad0aa43d901999c875ce00c232c1550ca91c9cbee73585d832a01d3a9757f033b0e7afbe798389d441845025ecfdde954521459112827789607bdc354e1e771209e013e60569fc61d9302818100a5c2f689a00758889120e98d85fbd129369af14bb44650c8bb130e0988e54d35e3762aa99211ee6d7179fd81e15bf7392a5b7db7973e5da09f24f9e0aad80af8a594aceb44bfa1893cbe529c58303a39409ad7495928d4c2db61902b1fe1848afcfbd4a9473deaf5083b0cf0663bd6858c3654fba15e9a98bb3d8520cd5243bb028180454a76808aa181a199893ae47b3022a4d49c825406a138cfb1f0ab3eab5cfeb5fc515a5d73fd508e03aadf3b00a95dc94ad8151b1ce95a5ca04a04ccbab44bf80dc632cc4eeb6f0c84561dc3eb4157a26fe1678cf2627f5e2357fd5b26fa71d0cedc75bafd53b166d01a24189d3b71d46476fa55ea6f87add361b6fea21b166102818067a328df4d35aa8de02732bef0494c31b759528a2191610acfa40f3fb8de9cd2977f9716e423dfed7f68652ea2470ca02a327fbc9c8c3a9fa540ca1644dac4a947655863d45cf7d3452e3d9a50acfe8a33315c6f1896a5c79ac03a122c61a4abfd963a15085cd71d12635128b0d2b2c256b2d5996a002b2a58cf13003a3f37e702818100b6d2d85f2d0fff90710f526ebd0e60ed9bd9a8079c2f67d1cccb4894ba0fcefeb02831f45753f1d11358628486f954988e9c7b37459bb80769b9eabdcb5672d876068d4f843424e4b80bc239bfe9357d6fa7e0fb60e48b365328cc29f9d5b9648f2d0b4d28b0c0cb013f99f463c94fc8ac2a5a3163aced4bad70bbfd86d6792b';

describe('Crypto test', () => {
	it('can encrypt and decrypt string with provided pin using AES', () => {
		const secret = 'little secret';
		const pin = '123456';
		const encrypted = crypto.encryptSecret(secret, pin);

		assert(encrypted !== secret, 'Encrypted value is the same as provided value');

		const decrypted = crypto.decryptSecret(encrypted, pin);

		expect(decrypted).toBe(secret);
	});

	it('can generate secret key from input', async () => {
		const keypair = crypto.generateKey();

		console.log(keypair.input);

		const { input } = crypto.inputToKeypair(keypair.input as string);

		expect(input).toBe(keypair.input);
	});

	it('can sign messages and verify signatures', async () => {
		const keypair = crypto.generateKey();
		crypto.set(keypair);

		const message = 'hello world';
		const { signature, publicKey } = await crypto.sign(message);

		expect(publicKey).toBe(keypair.publicKey);
		expect(await crypto.verify(signature, message)).toBe(true);
	});

	it('can sign messages and verify signatures with specific input', async () => {
		const keypair = crypto.inputToKeypair(privateInputKey);
		crypto.set(keypair);

		const keypair2 = crypto.inputToKeypair(privateInputKey);

		expect(keypair2.publicKey).toBe(keypair.publicKey);

		const message = '28004708';

		const { signature, publicKey } = await crypto.sign(message);

		expect(publicKey).toBe(keypair.publicKey);
		expect(await crypto.verify(signature, message)).toBe(true);

		console.log(signature);
		console.log(publicKey);
	});

	it('can encrypt and decrypt message with generated keys', async () => {
		const keypair = crypto.inputToKeypair(privateInputKey);
		crypto.set(keypair);

		const message = 'hello world';

		const encrypted = await crypto.encryptMessage({ data: message }, keypair.publicKey as string);
		const decrypted = await crypto.decryptMessage(encrypted);

		expect(message !== encrypted).toBe(true);
		expect(message).toEqual(decrypted);
	});
});
