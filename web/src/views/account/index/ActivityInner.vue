<script setup lang="ts">
import CardBox from '@/components/ui/CardBox.vue'
import BaseButtonConfirm from '@/components/ui/BaseButtonConfirm.vue'
import SortableName from '@/components/ui/SortableName.vue'
import PuppyLoader from '@/components/ui/PuppyLoader.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import ActivityRow from './ActivityRow.vue'
import { activity, killAll as killAllInner, kill } from '!/account'
import { computed, ref, watch } from 'vue'
import { mdiSearchWeb, mdiShieldOffOutline, mdiHistory } from '@mdi/js'
import { store as loginStore } from '!/auth/login'
import UniversalCheckbox from '@/components/ui/UniversalCheckbox.vue'
import type { Paginated, Session, ActivityQuery, Authenticated } from 'types'

const login = loginStore()
const paginated = ref<Paginated<Session>>()
const query = ref<ActivityQuery>({
  with_expired: false,
  sort: 'expires_at',
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
  paginated.value = await activity({ ...query.value })
}

const revokeAll = async () => {
  await killAllInner()
  query.value = { ...query.value, limit: 15, offset: 0 }
}

const revokeOne = async (id: string) => {
  await kill(id)
  await find()
}

watch(query, find, { deep: true, immediate: true })
</script>
<template>
  <CardBox>
    <div class="-mx-4 -mt-4 px-6 py-4 border-b border-brownish-100 dark:border-brownish-700/50 rounded-t-2xl">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div class="flex items-center gap-2">
          <BaseIcon :path="mdiHistory" :size="14" class="text-brownish-400 dark:text-brownish-500" />
          <p class="text-xs font-semibold uppercase tracking-wider text-brownish-400 dark:text-brownish-500">Login Sessions</p>
          <span v-if="total" class="text-xs font-medium bg-brownish-100 dark:bg-brownish-700 text-brownish-400 dark:text-brownish-300 px-2 py-0.5 rounded-full">{{ total }}</span>
        </div>

        <div class="flex flex-wrap items-center gap-2">
          <UniversalCheckbox
            name="with_expired"
            label="Show expired"
            v-model="query.with_expired"
          />
          <BaseButtonConfirm
            color="danger"
            label="Revoke all"
            confirm-label="Yes, revoke all"
            @confirm="revokeAll"
            :icon="mdiShieldOffOutline"
            :disabled="total === 0"
            :small="true"
          />
          <div class="relative">
            <input
              type="text"
              v-model="search"
              placeholder="Search by IP or device"
              @keyup.enter="query.search = search"
              class="h-[34px] w-44 sm:w-56 pl-3 pr-8 text-sm rounded-lg transition duration-150 ease-in-out bg-white dark:bg-brownish-800 border border-brownish-50 dark:border-brownish-600 text-brownish-900 dark:text-white placeholder-brownish-100/60 dark:placeholder-brownish-400 focus:outline-none focus:ring-2 focus:ring-offset-0 focus:ring-redish-400/60 dark:focus:ring-redish-500/50"
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
      </div>
    </div>

    <div class="overflow-x-auto -mx-4 -mb-4" v-if="paginated">
      <table v-if="paginated.data.length" class="w-full">
        <thead>
          <tr>
            <th>IP Address</th>
            <th>Device</th>
            <th><SortableName label="Signed in" name="created_at" v-model="query" /></th>
            <th><SortableName label="Last seen" name="updated_at" v-model="query" /></th>
            <th><SortableName label="Expires" name="expires_at" v-model="query" /></th>
            <th>Status</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <ActivityRow
            :authenticated="(login.authenticated as Authenticated)"
            :session="session"
            v-for="session in paginated.data"
            :key="session.id"
            @revoke="revokeOne(session.id)"
          />
        </tbody>
      </table>

      <div v-else class="px-6 py-16 text-center">
        <BaseIcon :path="mdiHistory" :size="32" class="text-brownish-300 dark:text-brownish-600 mx-auto mb-3" />
        <p class="text-sm text-brownish-400">No sessions found</p>
      </div>

      <div v-if="paginated.data.length" class="flex items-center justify-between px-4 py-3 border-t border-brownish-100 dark:border-brownish-700/50">
        <BaseButton label="← Previous" @click="previousPage" :disabled="disablePreviousPage" :small="true" />
        <span class="text-xs text-brownish-400">{{ rangeStart }}–{{ rangeEnd }} of {{ total }}</span>
        <BaseButton label="Next →" @click="nextPage" :disabled="disableNextPage" :small="true" />
      </div>
    </div>
    <PuppyLoader v-else />
  </CardBox>
</template>
