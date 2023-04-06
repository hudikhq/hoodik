<script lang="ts">
	import type { CreateUser } from '$stores/auth/register';
	import { AppField, AppCheckbox, AppForm, AppButton, RegisterData } from '$lib/form';
	import { A } from 'flowbite-svelte';
	import * as yup from 'yup';

	export const config = {
		initialValues: {
			email: '',
			password: '',
			confirm_password: '',
			checkbox: false
		} as Partial<CreateUser>,
		validationSchema: yup.object().shape({
			email: yup.string().required('Email is required').email('Email is invalid'),
			password: yup.string().required('Password is required'),
			confirm_password: yup
				.string()
				.required('Please confirm your password')
				.oneOf([yup.ref('password')], 'Passwords do not match'),
			checkbox: yup
				.bool()
				.required('Checkbox must be accepted')
				.oneOf([true], 'Checkbox must be accepted')
		})
	};
</script>

<main class="bg-gray-50 dark:bg-gray-900">
	<div
		class="flex flex-col items-center justify-center px-6 mx-autoo md:h-screen pt:mt-0 dark:bg-gray-900"
	>
		<A
			href="/"
			class="flex items-center justify-center mb-8 text-2xl font-semibold lg:mb-10 dark:text-white"
		>
			<span>Hoodik</span>
		</A>

		<RegisterData />
		<!-- Card -->
		<div class="w-full max-w-xl p-6 space-y-8 sm:p-8 bg-white rounded-lg shadow dark:bg-gray-800">
			<h1 class="text-2xl font-bold text-gray-900 dark:text-white">Create Account</h1>
			<AppForm {...config} action="/auth/register/key">
				<AppField
					label="Your email"
					type="text"
					name="email"
					id="email"
					placeholder="your@email.com"
				/>
				<AppField
					label="Your password"
					type="password"
					name="password"
					id="password"
					placeholder="••••••••"
				/>
				<AppField
					label="Confirm password"
					type="password"
					name="confirm_password"
					id="confirm_password"
					placeholder="••••••••"
				/>
				<AppCheckbox name="checkbox" label="I take all the responsibility" />

				<AppButton label="Next" type="submit" />

				<div class="text-sm font-medium text-gray-500 dark:text-gray-400">
					Already have an account? <A
						href="/auth/login"
						class="text-primary-700 hover:underline dark:text-primary-500">Login here</A
					>
				</div>
			</AppForm>
		</div>
	</div>
</main>
