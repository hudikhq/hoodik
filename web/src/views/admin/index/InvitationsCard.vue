<script setup lang="ts">
import CardBox from '@/components/ui/CardBox.vue'
import CardBoxComponentHeader from '@/components/ui/CardBoxComponentHeader.vue'
import SortableName from '@/components/ui/SortableName.vue'
import PuppyLoader from '@/components/ui/PuppyLoader.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import InvitationRow from './InvitationRow.vue'
import { index, expire } from '!/admin/invitations'
import { computed, ref, watch } from 'vue'
import type { Paginated, Search } from 'types/admin/invitations'
import { AppField } from '@/components/form'
import { mdiSearchWeb, mdiPlus } from '@mdi/js'
import UniversalCheckbox from '@/components/ui/UniversalCheckbox.vue'
import InviteUserModal from '@/components/modals/InviteUserModal.vue'

const props = defineProps<{
  class?: string
}>()

const paginated = ref<Paginated>()
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

const expireOne = async (id: string) => {
  await expire(id)
  await find()
}

watch(query, find, { deep: true, immediate: true })
</script>
<template>
  <InviteUserModal v-model="inviteModal" @confirm="find" />
  <div :class="props.class">
    <CardBox class="flex w-full">
      <CardBoxComponentHeader :title="`Invitations (${total})`">
        <div class="flex space-x-2 pt-2">
          <div class="mt-1 mr-2">
            <UniversalCheckbox
              name="with_expired"
              label="With Expired"
              v-model="query.with_expired"
            />
          </div>
          <div>
            <BaseButton
              confirm-label="Confirm"
              @click="inviteModal = true"
              :xs="true"
              :icon="mdiPlus"
              label="Invite User"
            />
          </div>

          <AppField
            name="search"
            placeholder="Search (Email)"
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
              <th class="text-left">
                <SortableName label="Created" name="created_at" v-model="query" />
              </th>
              <th class="text-left">
                <SortableName label="Expires" name="expires_at" v-model="query" />
              </th>
              <th></th>
            </tr>
          </thead>

          <tbody>
            <InvitationRow
              :invitation="invitation"
              v-for="invitation in paginated.invitations"
              :key="invitation.id"
              @expire="expireOne(invitation.id)"
            />
          </tbody>
        </table>

        <div class="flex justify-center mt-4">
          <BaseButton label="Previous" @click="previousPage" :disabled="disablePreviousPage" />
          <div class="m-2">
            {{ (query.offset || 0) + paginated.invitations.length }} / {{ paginated.total }}
          </div>
          <BaseButton label="Next Page" @click="nextPage" :disabled="disableNextPage" />
        </div>
      </div>
      <PuppyLoader v-else />
    </CardBox>
  </div>
</template>
