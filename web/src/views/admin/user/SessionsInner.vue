<script setup lang="ts">
import CardBox from '@/components/ui/CardBox.vue'
import BaseButtonConfirm from '@/components/ui/BaseButtonConfirm.vue'
import CardBoxComponentHeader from '@/components/ui/CardBoxComponentHeader.vue'
import SortableName from '@/components/ui/SortableName.vue'
import PuppyLoader from '@/components/ui/PuppyLoader.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import SessionRow from './SessionRow.vue'
import { index, killForUser, kill } from '!/admin/sessions'
import { computed, ref, watch } from 'vue'
import type { Search, Session } from 'types/admin/sessions'
import { AppField } from '@/components/form'
import { mdiSearchWeb, mdiDelete } from '@mdi/js'
import type { User } from 'types/admin/users'
import UniversalCheckbox from '@/components/ui/UniversalCheckbox.vue'
import type { Paginated } from 'types'

const props = defineProps<{
  user: User
}>()

const paginated = ref<Paginated<Session>>()
const query = ref<Search>({
  with_expired: true,
  sort: 'updated_at',
  order: 'desc',
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

const killOne = async (id: string) => {
  await kill(id)

  await find()
}

watch(query, find, { deep: true, immediate: true })
</script>
<template>
  <CardBox>
    <CardBoxComponentHeader :title="`User Sessions (${total})`" class="mb-4">
      <div class="flex space-x-2 pt-2">
        <div class="mt-1 mr-2">
          <UniversalCheckbox
            name="with_expired"
            label="With Expired"
            v-model="query.with_expired"
          />
        </div>
        <BaseButtonConfirm
          color="danger"
          label="Kill all sessions"
          confirm-label="Confirm"
          @confirm="killAll"
          :icon="mdiDelete"
          :disabled="total === 0"
        />

        <AppField
          name="search"
          placeholder="Search (IP, User Agent, ID)"
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
            <th class="text-left">Killed</th>
            <th class="text-left"></th>
          </tr>
        </thead>

        <tbody>
          <SessionRow
            :session="session"
            v-for="session in paginated.data"
            :key="session.id"
            @kill="killOne(session.id)"
          />
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
</template>
