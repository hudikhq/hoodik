<script setup lang="ts">
import SectionMain from '@/components/ui/SectionMain.vue'
import CardBox from '@/components/ui/CardBox.vue'
import CardBoxComponentHeader from '@/components/ui/CardBoxComponentHeader.vue'
import { index } from '!/admin/users'
import { ref, watch } from 'vue'
import type { User } from 'types'

const users = ref<User[]>([])
const query = ref({
  sort: undefined,
  order: undefined,
  search: undefined,
  limit: 15,
  offset: 0
})

const find = async () => {
  users.value = await index(query.value)
}

watch(query, find, { deep: true, immediate: true })
</script>
<template>
  <SectionMain>
    <CardBox>
      <CardBoxComponentHeader title="Users" class="mb-4" />

      <div class="overflow-x-auto">
        <table class="w-full">
          <thead>
            <tr>
              <th class="text-left">ID</th>
              <th class="text-left">Email</th>
              <th class="text-left">Role</th>
              <th class="text-left">Created</th>
              <th class="text-left">Updated</th>
            </tr>
          </thead>

          <tbody>
            <tr v-for="user in users" :key="user.id">
              <td>{{ user.id }}</td>
              <td>{{ user.email }}</td>
              <td>{{ user.role }}</td>
              <td>{{ user.created_at }}</td>
              <td>{{ user.updated_at }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </CardBox>
  </SectionMain>
</template>
