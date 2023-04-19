<script setup lang="ts">
import type { ListAppFile } from '@/stores/storage'

const props = defineProps<{
  parents: ListAppFile[]
}>()
</script>

<template>
  <nav class="flex" aria-label="Breadcrumb">
    <ol class="flex items-center space-x-2">
      <li>
        <component
          :is="props.parents.length ? 'router-link' : 'span'"
          to="/"
          class="text-gray-500 hover:text-gray-700"
          >root</component
        >
      </li>

      <template v-for="(parent, name, index) in props.parents" v-bind:key="index">
        <li v-if="index !== 0">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 20 20"
            fill="currentColor"
            class="text-gray-500 w-4 h-4"
          >
            <path
              fill-rule="evenodd"
              d="M7.293 15.707a1 1 0 0 1-1.414 0l-5-5a1 1 0 1 1 1.414-1.414L6 12.586l5.293-5.293a1 1 0 0 1 1.414 1.414l-5 5z"
              clip-rule="evenodd"
            />
          </svg>
        </li>

        <li>
          <span class="text-gray-500 hover:text-gray-700">{{ parent.metadata?.name }}</span>
        </li>
      </template>
    </ol>
  </nav>
</template>
