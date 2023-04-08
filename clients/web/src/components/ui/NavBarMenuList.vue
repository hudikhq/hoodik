<script setup lang="ts">
import NavBarItem from '@/components/ui/NavBarItem.vue'

defineProps({
  menu: {
    type: Array<{}>,
    default: () => []
  }
})

const emit = defineEmits(['menu-click'])

const menuClick = (event: Event, item: object) => {
  emit('menu-click', event, item)
}
</script>

<template>
  <NavBarItem v-for="(item, index) in menu" :key="index" :item="item" @menu-click="menuClick" />
</template>
