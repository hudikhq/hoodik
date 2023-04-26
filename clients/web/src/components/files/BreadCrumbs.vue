<script setup lang="ts">
import type { ListAppFile } from '@/stores/types'
import BaseButton from '../ui/BaseButton.vue'
import { computed } from 'vue'

const props = defineProps<{
  parents: ListAppFile[]
  parentId?: number
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
  <nav class="flex" aria-label="Breadcrumb">
    <ol class="flex items-center space-x-2">
      <li>
        <BaseButton
          to="/directory"
          label="/"
          :xs="true"
          color="lightDark"
          :disabled="!props.parents || !props.parents.length"
        />
      </li>

      <template v-for="(parent, index) in props.parents" v-bind:key="index">
        <li v-if="skipIndexes.indexOf(index) === -1">
          <BaseButton
            :to="`/directory/${parent.id}`"
            :xs="true"
            color="lightDark"
            :label="`${parent.metadata?.name}/`"
            :disabled="parent.id === props.parentId"
          />
        </li>
        <li v-else-if="skipIndexes[0] === index">...</li>
      </template>
    </ol>
  </nav>
</template>
