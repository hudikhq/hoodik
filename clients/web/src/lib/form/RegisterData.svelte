<script lang="ts">
	import { subscribe, type CreateUser } from '$stores/auth/register';

	let data: string = '';
	subscribe((d) => {
		data = JSON.stringify(d, null, 2);
	});
</script>

<div class="text-white mb-5">
	{data}
</div>
