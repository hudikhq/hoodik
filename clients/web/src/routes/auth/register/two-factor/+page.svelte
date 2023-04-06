<script lang="ts">
	import { A, Button } from 'flowbite-svelte';
	import * as yup from 'yup';
	import { AppField, AppForm, RegisterData } from '$lib/form';
	import type { CreateUser } from '$stores/auth/register';

	export let data: Partial<CreateUser>;

	let config: {
		[key: string]: unknown;
	} | null = null;

	export async function generateForm() {
		const secret = '123';

		data.secret = secret;
		data.token = '';

		config = {
			initialValues: data,
			validationSchema: yup.object().shape({
				secret: yup.string(),
				token: yup.string()
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
			<h1 class="text-2xl font-bold text-gray-900 dark:text-white">Setup two factor</h1>
			{#if config}
				<AppForm class="mt-8 space-y-6" {...config} let:values>
					<AppField
						label="Your two factor secret"
						name="secret"
						id="secret"
						placeholder=""
						disabled
						allowCopy={true}
					/>
					<Button type="submit">Create account</Button>
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
