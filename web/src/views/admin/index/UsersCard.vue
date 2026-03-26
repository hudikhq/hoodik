<script setup lang="ts">
import CardBox from '@/components/ui/CardBox.vue'
import CardBoxComponentHeader from '@/components/ui/CardBoxComponentHeader.vue'
import SortableName from '@/components/ui/SortableName.vue'
import PuppyLoader from '@/components/ui/PuppyLoader.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import UserRow from './UserRow.vue'
import { index } from '!/admin/users'
import { computed, ref, watch } from 'vue'
import type { Search, User } from 'types/admin/users'
import { mdiSearchWeb } from '@mdi/js'
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

const search = ref('')

const total = computed(() => paginated.value?.total || 0)

const disablePreviousPage = computed(() => {
  if (!paginated.value) return true
  return !query.value.offset
})

const disableNextPage = computed(() => {
  if (!paginated.value) return true
  return (query.value.offset || 0) + (query.value.limit || 15) > paginated.value.total
})

const rangeStart = computed(() => (query.value.offset || 0) + 1)
const rangeEnd = computed(() => (query.value.offset || 0) + (paginated.value?.data.length || 0))

const previousPage = () => {
  if (!paginated.value) return
  if (!query.value.offset) return
  query.value.offset = query.value.offset - (query.value.limit || 15)
}

const nextPage = () => {
  if (!paginated.value) return
  if (typeof query.value.offset === 'undefined') query.value.offset = 0
  query.value.offset = query.value.offset + (query.value.limit || 15)
}

const find = async () => {
  paginated.value = await index(query.value)
}

watch(query, find, { deep: true, immediate: true })
</script>
<template>
  <div class="flex" :class="props.class">
    <CardBox class="flex w-full">
      <CardBoxComponentHeader :title="`Users (${total})`">
        <div class="flex flex-wrap items-center gap-2 px-4 py-3">
          <div class="relative">
            <input
              type="text"
              v-model="search"
              placeholder="Search by ID or email"
              @keyup.enter="query.search = search"
              class="h-[34px] w-48 sm:w-60 pl-3 pr-8 text-sm rounded-lg transition duration-150 ease-in-out bg-white dark:bg-brownish-800 border border-brownish-50 dark:border-brownish-600 text-brownish-900 dark:text-white placeholder-brownish-100/60 dark:placeholder-brownish-400 focus:outline-none focus:ring-2 focus:ring-offset-0 focus:ring-redish-400/60 dark:focus:ring-redish-500/50"
            />
            <button
              type="button"
              @click="query.search = search"
              class="absolute right-2 top-1/2 -translate-y-1/2 text-brownish-400 hover:text-brownish-900 dark:hover:text-white transition-colors"
              aria-label="Search"
            >
              <BaseIcon :path="mdiSearchWeb" :size="15" />
            </button>
          </div>
        </div>
      </CardBoxComponentHeader>

      <div class="overflow-x-auto -mx-4 -mb-4" v-if="paginated">
        <table class="w-full">
          <thead>
            <tr>
              <th><SortableName label="Email" name="email" v-model="query" /></th>
              <th>TFA</th>
              <th>Role</th>
              <th>Email Verified</th>
              <th><SortableName label="Joined" name="created_at" v-model="query" /></th>
              <th>Last Active</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            <UserRow :user="user" v-for="user in paginated.data" :key="user.id" />
          </tbody>
        </table>

        <div class="flex items-center justify-between px-4 py-3 border-t border-brownish-100 dark:border-brownish-700/50">
          <BaseButton label="← Previous" @click="previousPage" :disabled="disablePreviousPage" :small="true" />
          <span class="text-xs text-brownish-400">{{ rangeStart }}–{{ rangeEnd }} of {{ total }}</span>
          <BaseButton label="Next →" @click="nextPage" :disabled="disableNextPage" :small="true" />
        </div>
      </div>
      <PuppyLoader v-else />
    </CardBox>
  </div>
</template>
