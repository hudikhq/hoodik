import { defineStore } from 'pinia'
import { ref } from 'vue'

export const store = defineStore('notifications', () => {
  const notifications = ref([])

  return {
    notifications
  }
})
