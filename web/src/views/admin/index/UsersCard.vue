<script setup lang="ts">
import CardBox from '@/components/ui/CardBox.vue'
import CardBoxComponentHeader from '@/components/ui/CardBoxComponentHeader.vue'
import SortableName from '@/components/ui/SortableName.vue'
import PuppyLoader from '@/components/ui/PuppyLoader.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import UserRow from './UserRow.vue'
import { index } from '!/admin/users'
import { computed, ref, watch } from 'vue'
import type { Search, User } from 'types/admin/users'
import { AppField } from '@/components/form'
import { mdiSearchWeb } from '@mdi/js'
import InviteUserModal from '@/components/modals/InviteUserModal.vue'
import type { Paginated } from 'types'

const props = defineProps<{
  class?: string
}>()

const paginated = ref<Paginated<User>>()
const query = ref<Search>({
  sort: 'created_at',
  order: 'desc',
  search: undefined,
  limit: 15,
  offset: 0
})

const inviteModal = ref(false)
const search = ref('')

const total = computed(() => {
  return paginated.value?.total || 0
})

const disablePreviousPage = computed(() => {
  if (!paginated.value) return true

  return !query.value.offset
})

const disableNextPage = computed(() => {
  if (!paginated.value) return true

  return (query.value.offset || 0) + (query.value.limit || 15) > paginated.value.total
})

const previousPage = () => {
  if (!paginated.value) return
  if (!query.value.offset) return

  query.value.offset = query.value.offset - (query.value.limit || 15)
}

const nextPage = () => {
  if (!paginated.value) return
  if (typeof query.value.offset === 'undefined') {
    query.value.offset = 0
  }

  query.value.offset = query.value.offset + (query.value.limit || 15)
}

const find = async () => {
  paginated.value = await index(query.value)
}

watch(query, find, { deep: true, immediate: true })
</script>
<template>
  <InviteUserModal v-model="inviteModal" />
  <div class="flex" :class="props.class">
    <CardBox class="flex w-full">
      <CardBoxComponentHeader :title="`Users (${total})`">
        <div class="flex space-x-2 pt-2">
          <AppField
            name="search"
            placeholder="Search (ID, Email)"
            v-model="search"
            class-add="text-sm pt-1 pl-1 pr-1 pb-1 h-[34px]"
            @confirm="query.search = search"
            :no-outer-margin="true"
          />

          <div>
            <BaseButton :icon="mdiSearchWeb" @click="query.search = search" :small="true" />
          </div>
        </div>
      </CardBoxComponentHeader>

      <div class="overflow-x-auto" v-if="paginated">
        <table class="w-full">
          <thead>
            <tr>
              <th class="text-left">
                <SortableName label="Email" name="email" v-model="query" />
              </th>
              <th class="text-left">Has TFA</th>
              <th class="text-left">Role</th>
              <th class="text-left">Email Activated</th>
              <th class="text-left">
                <SortableName label="Created" name="created_at" v-model="query" />
              </th>
              <th class="text-left">Last Active</th>
              <th></th>
            </tr>
          </thead>

          <tbody>
            <UserRow :user="user" v-for="user in paginated.data" :key="user.id" />
          </tbody>
        </table>

        <div class="flex justify-center mt-4">
          <BaseButton label="Previous" @click="previousPage" :disabled="disablePreviousPage" />
          <div class="m-2">
            {{ (query.offset || 0) + paginated.data.length }} / {{ paginated.total }}
          </div>
          <BaseButton label="Next Page" @click="nextPage" :disabled="disableNextPage" />
        </div>
      </div>
      <PuppyLoader v-else />
    </CardBox>
  </div>
</template>
