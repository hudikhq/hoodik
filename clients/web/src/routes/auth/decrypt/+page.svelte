<script lang="ts">
	import { Label, Input, A, Checkbox, Button } from 'flowbite-svelte';

	export async function login(e: Event) {}
</script>

<main class="bg-gray-50 dark:bg-gray-900">
	<div
		class="flex flex-col items-center justify-center px-6 pt-8 mx-auto md:h-screen pt:mt-0 dark:bg-gray-900"
	>
		<A
			href="/"
			class="flex items-center justify-center mb-8 text-2xl font-semibold lg:mb-10 dark:text-white"
		>
			<span>Hoodik</span>
		</A>
		<div class="w-full max-w-xl p-6 space-y-8 sm:p-8 bg-white rounded-lg shadow dark:bg-gray-800">
			<!-- <img src="" alt="" class="mr-4 h-11" /> -->
			<h1 class="text-2xl font-bold text-gray-900 dark:text-white">Decrypt your private key</h1>
			<form class="mt-8 space-y-6" on:submit|preventDefault>
				<div>
					<Label for="pin" class="mb-2">Enter your pin</Label>
					<Input type="password" name="pin" id="pin" placeholder="••••••••" required />
				</div>
				<Button type="submit">Decrypt</Button>
				<div class="text-sm font-medium text-gray-500 dark:text-gray-400">
					Not your account? <A
						href="/auth/login"
						class="text-primary-700 hover:underline dark:text-primary-500">Login</A
					>
				</div>
				<div class="text-sm font-medium text-gray-500 dark:text-gray-400">
					Not registered? <A
						href="/auth/register"
						class="text-primary-700 hover:underline dark:text-primary-500">Create account</A
					>
				</div>
			</form>
		</div>
	</div>
</main>
