<script setup lang="ts">
import AsideMenuItem from '@/components/ui/AsideMenuItem.vue'

defineProps({
  isDropdownList: Boolean,
  menu: {
    type: Array<Object>,
    required: true
  }
})

const emit = defineEmits(['menu-click'])

const menuClick = (event: Event, item: object) => {
  emit('menu-click', event, item)
}
</script>

<template>
  <ul>
    <AsideMenuItem
      v-for="(item, index) in menu"
      :key="index"
      :item="item"
      :is-dropdown-list="isDropdownList"
      @menu-click="menuClick"
    />
  </ul>
</template>
