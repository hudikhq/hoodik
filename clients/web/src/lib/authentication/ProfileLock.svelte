<script lang="ts">
	import { P, Button } from 'flowbite-svelte';
	export let site = {
		name: 'Flowbite',
		img: 'https://flowbite-admin-dashboard.vercel.app/images/logo.svg',
		link: 'https://flowbite-admin-dashboard.vercel.app/',
		imgAlt: 'FlowBite Logo'
	};
	export let user = {
		name: 'Bonnie Green',
		img: 'https://flowbite-admin-dashboard.vercel.app/images/users/bonnie-green.png',
		imgAlt: 'Bonnie Green'
	};
	export let btnTitle = 'Reset password';
	export let pageDescription = 'Better to be safe than sorry.';
	export let mainClass = 'bg-gray-50 dark:bg-gray-900';
	export let mainDivClass =
		'flex flex-col items-center justify-center px-6 pt-8 mx-auto md:h-screen pt:mt-0 dark:bg-gray-900';
	export let siteLinkClass =
		'flex items-center justify-center mb-8 text-2xl font-semibold lg:mb-10 dark:text-white';
	export let siteImgClass = 'mr-4 h-11';
	export let cardDivClass =
		'w-full bg-white rounded-lg shadow md:mt-0 sm:max-w-md xl:p-0 dark:bg-gray-800';
	export let cardH1Class = 'mb-3 text-2xl font-bold text-gray-900 dark:text-white';
	export let cardDiv2Class = 'w-full p-6 sm:p-8';
	export let cardDiv3Class = 'flex space-x-4';
	export let userImgClass = 'w-8 h-8 rounded-full';
</script>

<main class={mainClass}>
	<div class={mainDivClass}>
		<a href={site.link} class={siteLinkClass}>
			<img src={site.img} class={siteImgClass} alt={site.imgAlt} />
			<span>{site.name}</span>
		</a>
		<!-- Card -->
		<div class={cardDivClass}>
			<div class={cardDiv2Class}>
				<div class={cardDiv3Class}>
					<img class={userImgClass} src={user.img} alt={user.imgAlt} />
					<h1 class={cardH1Class}>{user.name}</h1>
				</div>
				<P class="text-base font-normal text-gray-500 dark:text-gray-400">
					{pageDescription}
				</P>
				<form class="mt-8 space-y-6" on:submit|preventDefault>
					<slot />
					<Button type="submit">{btnTitle}</Button>
				</form>
			</div>
		</div>
	</div>
</main>
