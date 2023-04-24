<script setup lang="ts">
import type { ListAppFile } from '@/stores/storage/types'

const props = defineProps<{
  parents: ListAppFile[]
}>()
</script>

<template>
  <nav class="flex" aria-label="Breadcrumb">
    <ol class="flex items-center space-x-2">
      <li>
        <router-link to="/" class="text-gray-500 hover:text-gray-700"> root </router-link>
      </li>

      <template v-for="(parent, index) in props.parents" v-bind:key="index">
        <span class="text-gray-500">/</span>

        <li>
          <router-link :to="`/${parent.id}`" class="text-gray-500 hover:text-gray-700">{{
            parent.metadata?.name
          }}</router-link>
        </li>
      </template>
    </ol>
  </nav>
</template>
