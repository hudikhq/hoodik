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
import {
  mdiDelete,
  mdiRefresh,
  mdiHuman,
  mdiPencil,
  mdiClose,
  mdiShieldOff,
  mdiShieldCheck,
  mdiAlertCircleOutline,
  mdiChevronLeft,
  mdiContentCopy,
  mdiCheck
} from '@mdi/js'
import { formatPrettyDate, formatSize } from '!/index'
import BaseButtonConfirm from '@/components/ui/BaseButtonConfirm.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import SessionsInner from './user/SessionsInner.vue'
import { useTitle } from '@vueuse/core'
import QuotaSlider from '@/components/ui/QuotaSlider.vue'

const title = useTitle()
const route = useRoute()
const router = useRouter()
const data = ref<Response>()
const editQuota = ref(false)
const currentQuota = ref()
const copiedField = ref<string | null>(null)

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

const initials = computed(() => user.value?.email[0]?.toUpperCase() || '?')

const quota = computed(() => {
  if (!user.value?.quota && typeof user.value?.quota !== 'number') return null
  return formatSize(user.value.quota)
})

const createdAt = computed(() => {
  if (!user.value) return null
  return formatPrettyDate(user.value.created_at)
})

const emailVerifiedAt = computed(() => {
  if (!user.value?.email_verified_at) return null
  return formatPrettyDate(user.value?.email_verified_at)
})

const lastActiveAt = computed(() => {
  if (!user.value?.last_session?.updated_at) return null
  return formatPrettyDate(user.value?.last_session?.updated_at)
})

const openQuotaEdit = () => {
  currentQuota.value = user.value?.quota
  editQuota.value = true
}

const closeQuotaEdit = () => {
  if (user.value) user.value.quota = currentQuota.value
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
  router.push({ name: 'manage-users' })
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

async function copyToClipboard(value: string, field: string) {
  await navigator.clipboard.writeText(value)
  copiedField.value = field
  setTimeout(() => { copiedField.value = null }, 2000)
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
    <SectionMain v-if="data && user">
      <!-- Page header: breadcrumb + user identity -->
      <div class="mb-4">
        <BaseButton
          :icon="mdiChevronLeft"
          label="All Users"
          :small="true"
          :to="{ name: 'manage-users' }"
          class="mb-3"
        />

        <CardBox>
          <div class="-mx-4 -my-4 px-6 py-5">
            <div class="flex items-start gap-4">
              <div class="w-14 h-14 rounded-full bg-redish-500/15 dark:bg-redish-500/20 text-redish-500 dark:text-redish-400 flex items-center justify-center text-2xl font-bold shrink-0 select-none">
                {{ initials }}
              </div>
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2 flex-wrap">
                  <h1 class="text-lg font-semibold truncate">{{ user.email }}</h1>
                  <BaseButton :icon="mdiRefresh" :xs="true" @click="get" title="Refresh" />
                </div>
                <div class="flex flex-wrap items-center gap-2 mt-2">
                  <span v-if="user.role" class="inline-flex items-center text-xs font-semibold uppercase tracking-wider bg-orangy-400/15 text-orangy-400 px-2 py-0.5 rounded-full">
                    {{ user.role }}
                  </span>
                  <span v-else class="inline-flex text-xs bg-brownish-100 dark:bg-brownish-700 text-brownish-400 dark:text-brownish-300 px-2 py-0.5 rounded-full">
                    user
                  </span>
                  <span v-if="emailVerifiedAt" class="inline-flex items-center gap-1 text-xs text-greeny-500 dark:text-greeny-400">
                    <BaseIcon :path="mdiShieldCheck" :size="13" />
                    email verified
                  </span>
                  <span v-else class="inline-flex items-center gap-1 text-xs text-redish-500">
                    <BaseIcon :path="mdiAlertCircleOutline" :size="13" />
                    email unverified
                  </span>
                  <span v-if="user.secret" class="inline-flex items-center gap-1 text-xs text-greeny-500 dark:text-greeny-400">
                    <BaseIcon :path="mdiShieldCheck" :size="13" />
                    TFA on
                  </span>
                  <span class="text-xs text-brownish-400">
                    · joined {{ createdAt }}
                    <span v-if="lastActiveAt"> · last active {{ lastActiveAt }}</span>
                  </span>
                </div>
              </div>
            </div>
          </div>
        </CardBox>
      </div>

      <!-- Main content: two columns -->
      <div class="flex flex-col sm:flex-row gap-4 mb-4">
        <!-- Left: Storage -->
        <CardBox class="w-full sm:w-1/2">
          <CardBoxComponentHeader title="Storage">
            <div v-if="!editQuota" class="flex items-center px-4 py-3">
              <BaseButton :icon="mdiPencil" :small="true" label="Edit quota" @click="openQuotaEdit" />
            </div>
          </CardBoxComponentHeader>

          <div class="-mx-4 -mb-4">
            <!-- Quota display / editor -->
            <div class="px-4 py-4 border-b border-brownish-100 dark:border-brownish-700/50">
              <div v-if="!editQuota" class="flex items-center justify-between">
                <div>
                  <p class="text-xs font-medium uppercase tracking-wider text-brownish-400 dark:text-brownish-500 mb-0.5">Storage Quota</p>
                  <p class="text-sm">{{ quota ?? 'default' }}</p>
                </div>
              </div>
              <div v-else class="space-y-3">
                <p class="text-xs font-medium uppercase tracking-wider text-brownish-400 dark:text-brownish-500">Edit Storage Quota</p>
                <QuotaSlider v-model="user.quota" />
                <div class="flex items-center gap-2 pt-1">
                  <BaseButtonConfirm
                    :small="true"
                    label="Save quota"
                    confirm-label="Confirm"
                    @confirm="updateQuota"
                  />
                  <BaseButton
                    :icon="mdiClose"
                    :small="true"
                    @click="closeQuotaEdit"
                    color="danger"
                    label="Cancel"
                  />
                </div>
              </div>
            </div>

            <!-- Storage breakdown -->
            <div class="px-4 py-3">
              <p class="text-xs font-medium uppercase tracking-wider text-brownish-400 dark:text-brownish-500 mb-3">Storage Breakdown</p>
              <StatsTable :data="data.stats" />
            </div>
          </div>
        </CardBox>

        <!-- Right: Account controls + danger zone -->
        <div class="w-full sm:w-1/2 flex flex-col gap-4">
          <!-- Account controls -->
          <CardBox>
            <CardBoxComponentHeader title="Account Controls" />

            <div class="-mx-4 -mb-4">
              <!-- Two-factor auth -->
              <div class="flex items-center justify-between px-4 py-4 border-b border-brownish-100 dark:border-brownish-700/50">
                <div>
                  <p class="text-sm font-medium">Two-Factor Auth</p>
                  <p class="text-xs mt-0.5">
                    <span v-if="user.secret" class="text-greeny-500 dark:text-greeny-400">Enabled</span>
                    <span v-else class="text-brownish-400">Not enabled</span>
                  </p>
                </div>
                <BaseButtonConfirm
                  v-if="user.secret"
                  :icon="mdiShieldOff"
                  color="danger"
                  :small="true"
                  label="Disable TFA"
                  confirm-label="Yes, disable"
                  @confirm="disableTfa"
                />
                <span v-else class="text-xs text-brownish-400 italic">—</span>
              </div>

              <!-- Role management -->
              <div class="flex items-center justify-between px-4 py-4">
                <div>
                  <p class="text-sm font-medium">Role</p>
                  <p class="text-xs mt-0.5">
                    <span v-if="user.role" class="font-semibold uppercase tracking-wider text-orangy-400">{{ user.role }}</span>
                    <span v-else class="text-brownish-400">Regular user</span>
                  </p>
                </div>
                <div class="flex items-center gap-2">
                  <BaseButtonConfirm
                    v-if="!user.role"
                    :icon="mdiHuman"
                    :small="true"
                    label="Make admin"
                    confirm-label="Yes, promote"
                    @confirm="setRole('admin')"
                  />
                  <BaseButtonConfirm
                    v-else
                    :icon="mdiDelete"
                    color="danger"
                    :small="true"
                    label="Remove role"
                    confirm-label="Yes, demote"
                    @confirm="setRole()"
                  />
                </div>
              </div>
            </div>
          </CardBox>

          <!-- Encryption keys -->
          <CardBox>
            <CardBoxComponentHeader title="Encryption Keys" />
            <div class="-mx-4 -mb-4 px-4 py-3 space-y-3">
              <div>
                <p class="text-xs font-medium uppercase tracking-wider text-brownish-400 dark:text-brownish-500 mb-1">Public Key</p>
                <div class="flex items-start gap-2">
                  <code class="flex-1 font-mono text-xs break-all text-brownish-400 dark:text-brownish-300 leading-relaxed">{{ user.pubkey }}</code>
                  <button @click="copyToClipboard(user.pubkey, 'pubkey')" class="shrink-0 mt-0.5 text-brownish-400 hover:text-white transition-colors">
                    <BaseIcon :path="copiedField === 'pubkey' ? mdiCheck : mdiContentCopy" :size="14" />
                  </button>
                </div>
              </div>
              <div class="pb-1">
                <p class="text-xs font-medium uppercase tracking-wider text-brownish-400 dark:text-brownish-500 mb-1">Fingerprint</p>
                <div class="flex items-start gap-2">
                  <code class="flex-1 font-mono text-xs break-all text-brownish-400 dark:text-brownish-300">{{ user.fingerprint }}</code>
                  <button @click="copyToClipboard(user.fingerprint, 'fingerprint')" class="shrink-0 mt-0.5 text-brownish-400 hover:text-white transition-colors">
                    <BaseIcon :path="copiedField === 'fingerprint' ? mdiCheck : mdiContentCopy" :size="14" />
                  </button>
                </div>
              </div>
            </div>
          </CardBox>

          <!-- Danger zone -->
          <CardBox>
            <div class="-mx-4 -my-4 border border-redish-500/30 rounded-2xl overflow-hidden">
              <div class="px-4 py-3 border-b border-redish-500/20 bg-redish-500/5">
                <div class="flex items-center gap-2">
                  <BaseIcon :path="mdiAlertCircleOutline" :size="16" class="text-redish-500 shrink-0" />
                  <p class="text-sm font-semibold text-redish-500">Danger Zone</p>
                </div>
              </div>
              <div class="px-4 py-4 flex items-center justify-between gap-4">
                <div>
                  <p class="text-sm font-medium">Delete this account</p>
                  <p class="text-xs text-brownish-400 mt-0.5">Permanently deletes the user and all their files. This cannot be undone.</p>
                </div>
                <BaseButtonConfirm
                  :icon="mdiDelete"
                  color="danger"
                  :small="true"
                  label="Delete user"
                  confirm-label="Yes, delete"
                  @confirm="remove"
                  class="shrink-0"
                />
              </div>
            </div>
          </CardBox>
        </div>
      </div>

      <!-- Sessions table -->
      <SessionsInner :user="user" />
    </SectionMain>
  </LayoutAdminWithLoader>
</template>
