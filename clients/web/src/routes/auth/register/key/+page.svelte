<script lang="ts">
	import { A, Button } from 'flowbite-svelte';
	import * as yup from 'yup';
	import { rsa } from '$stores/cryptfns';
	import { AppForm, AppTextarea, AppCheckbox, RegisterData } from '$lib/form';
	import type { CreateUser } from '$stores/auth/register';

	export let data: Partial<CreateUser>;

	console.debug(data);

	let config: {
		[key: string]: unknown;
	} | null = null;

	export async function generateForm() {
		const keypair = await rsa.generateKeyPair();

		config = {
			initialValues: {
				...data,
				pubkey: keypair.publicKey,
				fingerprint: keypair.fingerprint,
				unencrypted_private_key: keypair.input,
				store_private_key: true,
				i_have_stored_my_private_key: false
			} as Partial<CreateUser>,
			validationSchema: yup.object().shape({
				pubkey: yup.string().required('Public key is required'),
				fingerprint: yup.string().required('Fingerprint is required'),
				unencrypted_private_key: yup.string(),
				store_private_key: yup.bool().default(true),
				i_have_stored_my_private_key: yup
					.bool()
					.default(false)
					.required('You must confirm that you have stored your private key')
					.oneOf([true], 'You must confirm that you have stored your private key')
			})
		};
	}
	generateForm();
</script>

<main class="bg-gray-50 dark:bg-gray-900">
	<div class="flex flex-col items-center justify-center px-6 mx-auto pt:mt-0 dark:bg-gray-900">
		<A
			href="/"
			class="flex items-center justify-center mb-8 text-2xl font-semibold lg:mb-10 dark:text-white"
		>
			<span>Hoodik</span>
		</A>

		<RegisterData />
		<!-- Card -->
		<div class="w-full max-w-xl p-6 space-y-8 sm:p-8 bg-white rounded-lg shadow dark:bg-gray-800">
			<h1 class="text-2xl font-bold text-gray-900 dark:text-white">Your private key</h1>
			{#if config}
				<AppForm
					class="mt-8 space-y-6"
					{...config}
					let:values
					method="POST"
					action="/auth/register/two-factor"
				>
					<div class="flex items-start">
						<div class="flex items-center h-5">
							<p class="text-sm text-red-500 dark:text-red-400">
								<strong>This is the last time we'll show you your key!</strong> Store it somewhere safe.
							</p>
						</div>
					</div>
					<AppTextarea
						rows="28"
						class="text-xs"
						label="Your private key"
						name="unencrypted_private_key"
						id="unencrypted_private_key"
						placeholder=""
						disabled
						allowCopy={true}
					/>
					<AppCheckbox label="Encrypt and store my private key" name="store_private_key" />
					<div class="flex items-start">
						<div class="flex items-center h-5 mb-1">
							{#if values.store_private_key}
								<p class="text-sm text-green-500 dark:text-green-400">
									Your private key will be encrypted with your password and then it will be sent and
									stored on the backend server. This will allow you to login simply with your email
									and password.
								</p>
							{:else}
								<p class="text-sm text-red-500 dark:text-red-400">
									Not storing your private key on the server means you have to protect it yourself.
									Every time you login you will have to enter your private key in order to be able
									to access your files.
								</p>
							{/if}
						</div>
					</div>
					<AppCheckbox label="I have stored my private key" name="i_have_stored_my_private_key" />
					<Button type="submit">Next</Button>
					<div class="text-sm font-medium text-gray-500 dark:text-gray-400">
						Already have an account? <A
							href="/auth/login"
							class="text-primary-700 hover:underline dark:text-primary-500">Login here</A
						>
					</div>
				</AppForm>
			{/if}
		</div>
	</div>
</main>
