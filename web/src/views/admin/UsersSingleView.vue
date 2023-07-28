<script setup lang="ts">
import SectionMain from '@/components/ui/SectionMain.vue'
import CardBox from '@/components/ui/CardBox.vue'
import CardBoxComponentHeader from '@/components/ui/CardBoxComponentHeader.vue'
import LayoutAdminWithLoader from '@/layouts/LayoutAdminWithLoader.vue'
import StatsTable from '@/components/files/stats/StatsTable.vue'
import type { Response, User } from 'types/admin/users'
import { users } from '!/admin'
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { mdiDelete, mdiRefresh, mdiHuman, mdiPencil, mdiClose } from '@mdi/js'
import { formatPrettyDate, formatSize } from '!/index'
import BaseButtonConfirm from '@/components/ui/BaseButtonConfirm.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import SessionsInner from './user/SessionsInner.vue'
import { useTitle } from '@vueuse/core'
import QuotaSlider from '@/components/ui/QuotaSlider.vue'

const title = useTitle()
const route = useRoute()
const router = useRouter()
const data = ref<Response>()
const editQuota = ref(false)
const currentQuota = ref()

const user = computed({
  get(): User | null {
    if (!data.value) return null

    return data.value.user
  },
  set(value: User | null) {
    if (!data.value) return
    if (!value) return

    data.value.user = value
  }
})

const quota = computed(() => {
  if (!user.value?.quota && typeof user.value?.quota !== 'number') return 'default'

  return formatSize(user.value.quota)
})

const createdAt = computed(() => {
  if (!user.value) return null

  return formatPrettyDate(user.value.created_at)
})

const emailVerifiedAt = computed(() => {
  if (!user.value?.email_verified_at) return 'not-verified'

  return formatPrettyDate(user.value?.email_verified_at)
})

const lastActiveAt = computed(() => {
  if (!user.value?.last_session?.updated_at) return 'no data'
  return formatPrettyDate(user.value?.last_session?.updated_at)
})

const openQuotaEdit = () => {
  currentQuota.value = user.value?.quota
  editQuota.value = true
}

const closeQuotaEdit = () => {
  if (user.value) {
    user.value.quota = currentQuota.value
  }

  editQuota.value = false
}

const updateQuota = async () => {
  if (!user.value) return

  await update()

  editQuota.value = false
}

const update = async () => {
  if (!user.value) return

  const response = await users.update(user.value.id, {
    role: user.value.role,
    quota: user.value.quota
  })

  user.value = response.user
}

const disableTfa = async () => {
  if (!user.value) return

  data.value = await users.disableTfa(user.value.id)
}

const remove = async () => {
  if (!user.value) return

  await users.remove(user.value.id)

  router.push({
    name: 'admin-users'
  })
}

const get = async () => {
  if (!user.value) return

  data.value = await users.get(user.value.id)
}

const setRole = async (role?: 'admin') => {
  if (!user.value) return
  user.value.role = role

  await update()
}

watch(
  () => route.params.id,
  async (id: string | string[]) => {
    id = Array.isArray(id) ? id[0] : id

    data.value = await users.get(id)

    title.value = `${data.value.user.email} -- ${window.defaultDocumentTitle}`
  },
  { immediate: true }
)
</script>
<template>
  <LayoutAdminWithLoader>
    <SectionMain v-if="data">
      <div class="flex space-x-2">
        <CardBox class="sm:w-1/2" v-if="user">
          <CardBoxComponentHeader
            title="User details"
            :button-icon="mdiRefresh"
            @button-click="get"
          >
            <BaseButtonConfirm
              :icon="mdiDelete"
              color="danger"
              class="mt-1"
              small
              rounded-full
              label="Delete user"
              confirm-label="Confirm"
              @confirm="remove"
            />
          </CardBoxComponentHeader>

          <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
            <div class="flex flex-col w-1/2">Email</div>
            <div class="flex flex-col w-1/2">{{ user.email }}</div>
          </div>
          <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
            <div class="flex flex-col w-1/2">Storage Quota</div>
            <div class="flex flex-col w-1/2" v-if="!editQuota">
              <div>
                <BaseButton
                  :icon="mdiPencil"
                  :xs="true"
                  rounded-full
                  :label="quota"
                  @click="openQuotaEdit"
                />
              </div>
            </div>
            <div class="flex flex-col w-1/2" v-else>
              <div class="mb-2">
                <QuotaSlider v-model="user.quota" />
              </div>

              <div class="w-full justify-end">
                <BaseButton
                  :icon="mdiClose"
                  :xs="true"
                  rounded-full
                  @click="closeQuotaEdit"
                  class="float-right"
                  color="danger"
                />

                <BaseButtonConfirm
                  :icon="mdiDelete"
                  small
                  rounded-full
                  label="Save"
                  confirm-label="Confirm"
                  @confirm="updateQuota"
                  class="mr-2 float-right"
                />
              </div>
            </div>
          </div>
          <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
            <div class="flex flex-col w-1/2">Email Verified</div>
            <div class="flex flex-col w-1/2">{{ emailVerifiedAt }}</div>
          </div>
          <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
            <div class="flex flex-col w-6/12">Has two factor</div>
            <div class="flex flex-col w-6/2">
              <BaseButtonConfirm
                :icon="mdiDelete"
                color="danger"
                small
                rounded-full
                label="Disable TFA"
                confirm-label="Confirm"
                @confirm="disableTfa"
                v-if="user.secret"
              />
              <BaseButton v-else label="No" :small="true" class="cursor-auto" />
            </div>
          </div>
          <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
            <div class="flex flex-col w-6/12">User role</div>
            <div class="flex flex-col w-6/2">
              <BaseButtonConfirm
                v-if="!user.role"
                :icon="mdiHuman"
                small
                rounded-full
                label="Set as admin"
                confirm-label="Confirm"
                @confirm="setRole('admin')"
              />
              <BaseButtonConfirm
                v-if="user.role"
                :icon="mdiDelete"
                small
                rounded-full
                :label="`Remove ${user.role}`"
                confirm-label="Confirm"
                @confirm="setRole()"
              />
            </div>
          </div>
          <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
            <div class="flex flex-col w-1/2">Created</div>
            <div class="flex flex-col w-1/2">{{ createdAt }}</div>
          </div>
          <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
            <div class="flex flex-col w-1/2">Last active</div>
            <div class="flex flex-col w-1/2">{{ lastActiveAt }}</div>
          </div>
          <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
            <div class="flex flex-col w-1/2">Public key</div>
            <div class="flex flex-col w-1/2 text-xs">{{ user.pubkey }}</div>
          </div>
          <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
            <div class="flex flex-col w-1/2">Fingerprint</div>
            <div class="flex flex-col w-1/2 text-xs">{{ user.fingerprint }}</div>
          </div>
        </CardBox>

        <CardBox class="sm:w-1/2">
          <CardBoxComponentHeader title="Storage usage" class="mb-4" />

          <StatsTable :data="data.stats" />
        </CardBox>
      </div>

      <div class="mt-2">
        <SessionsInner v-if="user" :user="user" />
      </div>
    </SectionMain>
  </LayoutAdminWithLoader>
</template>
