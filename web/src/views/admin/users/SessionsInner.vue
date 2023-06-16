<script setup lang="ts">
import CardBox from '@/components/ui/CardBox.vue'
import BaseButtonConfirm from '@/components/ui/BaseButtonConfirm.vue'
import CardBoxComponentHeader from '@/components/ui/CardBoxComponentHeader.vue'
import SortableName from '@/components/ui/SortableName.vue'
import PuppyLoader from '@/components/ui/PuppyLoader.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import SessionRow from './SessionRow.vue'
import { index, killForUser } from '!/admin/sessions'
import { computed, ref, watch } from 'vue'
import type { Paginated, Search } from 'types/admin/sessions'
import { AppField } from '@/components/form'
import { mdiSearchWeb, mdiDelete } from '@mdi/js'
import type { User } from 'types/admin/users'
import UniversalCheckbox from '@/components/ui/UniversalCheckbox.vue'

const props = defineProps<{
  user: User
}>()

const paginated = ref<Paginated>()
const query = ref<Search>({
  with_deleted: false,
  with_expired: false,
  sort: undefined,
  order: undefined,
  search: undefined,
  limit: 15,
  offset: 0
})

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
  paginated.value = await index({ ...query.value, user_id: props.user.id })
}

const killAll = async () => {
  await killForUser(props.user.id)

  query.value = { ...query.value, limit: 15, offset: 0 }
}

watch(query, find, { deep: true, immediate: true })
</script>
<template>
  <CardBox>
    <CardBoxComponentHeader :title="`User Sessions (${total})`" class="mb-4">
      <div class="flex">
        <BaseButtonConfirm
          color="danger"
          label="Kill all sessions"
          confirm-label="Confirm"
          @confirm="killAll"
          :icon="mdiDelete"
          class="mr-2 mt-0.5"
          :disabled="total === 0"
        />

        <AppField name="search" placeholder="Search (IP, User Agent, ID)" v-model="search" />

        <BaseButton
          :icon="mdiSearchWeb"
          @click="query.search = search"
          class="ml-1 mt-0.5 h-10 w-10 rounded-lg"
        />
      </div>
    </CardBoxComponentHeader>

    <div class="flex justify-start space-x-2">
      <div class="mt-2">
        <UniversalCheckbox name="with_killed" label="With Killed" v-model="query.with_deleted" />
      </div>
      <div class="mt-2">
        <UniversalCheckbox name="with_expired" label="With Expired" v-model="query.with_expired" />
      </div>
    </div>

    <div class="overflow-x-auto" v-if="paginated">
      <table class="w-full">
        <thead>
          <tr>
            <th class="text-left">
              <SortableName label="Email" name="email" v-model="query" />
            </th>
            <th class="text-left">IP</th>
            <th class="text-left">User Agent</th>
            <th class="text-left">
              <SortableName label="Created" name="created_at" v-model="query" />
            </th>
            <th class="text-left">
              <SortableName label="Last update" name="updated_at" v-model="query" />
            </th>
            <th class="text-left">
              <SortableName label="Expires" name="expires_at" v-model="query" />
            </th>
          </tr>
        </thead>

        <tbody>
          <SessionRow :session="session" v-for="session in paginated.sessions" :key="session.id" />
        </tbody>
      </table>

      <div class="flex justify-center mt-4">
        <BaseButton label="Previous" @click="previousPage" :disabled="disablePreviousPage" />
        <div class="m-2">
          {{ (query.offset || 0) + paginated.sessions.length }} / {{ paginated.total }}
        </div>
        <BaseButton label="Next Page" @click="nextPage" :disabled="disableNextPage" />
      </div>
    </div>
    <PuppyLoader v-else />
  </CardBox>
</template>
