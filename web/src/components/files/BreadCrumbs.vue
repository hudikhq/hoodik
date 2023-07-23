<script setup lang="ts">
import type { AppFile } from 'types'
import BaseButton from '../ui/BaseButton.vue'
import { computed } from 'vue'

const props = defineProps<{
  parents: AppFile[]
  parentId?: string
}>()

const skipIndexes = computed<number[]>(() => {
  if (props.parents.length < 3) {
    return []
  } else {
    return props.parents.slice(1, props.parents.values.length - 2).map((_, index) => index + 1)
  }
})
</script>

<template>
  <nav aria-label="Breadcrumb">
    <ol class="flex mb-2">
      <li>
        <BaseButton
          :to="{ name: 'files' }"
          label="My Files"
          :disabled="!props.parents || !props.parents.length"
          class="pl-1 pr-1 text-lg"
        />
      </li>

      <template v-for="(parent, index) in props.parents" v-bind:key="index">
        <li v-if="skipIndexes.indexOf(index) === -1">
          <span> / </span>
          <BaseButton
            :to="{ name: 'files', params: { file_id: parent.id } }"
            :label="`${parent.name || '...'}`"
            class="pl-1 pr-1 text-lg"
          />
        </li>
        <li v-else-if="skipIndexes[0] === index">...</li>
      </template>
    </ol>
  </nav>
</template>
