<script setup lang="ts">
import { formatPrettyDate } from '!/index'
import type { User } from 'types'
import { computed } from 'vue'
import { mdiDelete, mdiPassport, mdiLock } from '@mdi/js'
import CardBox from '@/components/ui/CardBox.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import CardBoxComponentHeader from '@/components/ui/CardBoxComponentHeader.vue'

const props = defineProps<{
  user: User
}>()

const emits = defineEmits(['disableTfa', 'enableTfa'])

const emailVerifiedAt = computed(() => {
  if (props.user.email_verified_at) {
    return formatPrettyDate(props.user.email_verified_at)
  }
  return 'n/a'
})

const createdAt = computed(() => {
  return formatPrettyDate(props.user.created_at)
})
</script>
<template>
  <CardBox class="sm:w-1/2" v-if="user">
    <CardBoxComponentHeader title="My details">
      <div>
        <BaseButton
          :icon="mdiPassport"
          class="mt-1"
          :small="true"
          rounded-full
          label="Change password"
          :to="{ name: 'account-change-password' }"
        />
      </div>
    </CardBoxComponentHeader>

    <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
      <div class="flex flex-col w-1/2">Email</div>
      <div class="flex flex-col w-1/2">{{ user.email }}</div>
    </div>
    <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
      <div class="flex flex-col w-1/2">Email Verified</div>
      <div class="flex flex-col w-1/2">{{ emailVerifiedAt }}</div>
    </div>
    <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
      <div class="flex flex-col w-6/12">Has two factor</div>
      <div class="flex flex-col w-6/2">
        <BaseButton
          :icon="mdiDelete"
          color="danger"
          small
          rounded-full
          label="Disable TFA"
          @click="emits('disableTfa')"
          v-if="user.secret"
        />
        <BaseButton
          v-else
          :icon="mdiLock"
          color="info"
          small
          rounded-full
          label="Enable TFA"
          @click="emits('enableTfa')"
        />
      </div>
    </div>
    <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
      <div class="flex flex-col w-6/12">My role</div>
      <div class="flex flex-col w-6/2">
        {{ user.role || 'regular user' }}
      </div>
    </div>
    <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
      <div class="flex flex-col w-1/2">Created</div>
      <div class="flex flex-col w-1/2">{{ createdAt }}</div>
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
</template>
