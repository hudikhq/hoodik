<script setup lang="ts">
import { computed } from 'vue'
import SharingPeopleAddRoleChips from '@/components/shares/SharingPeopleAddRoleChips.vue'
import type { ShareableGroup } from '@/components/shares/composables/useSharingPeopleAdd'
import type { ShareRole } from 'types'

const props = defineProps<{
  group: ShareableGroup
  role: ShareRole
  submitting: boolean
  readOnly?: boolean
  disableCoOwner?: boolean
}>()

const emit = defineEmits<{
  (e: 'update:role', value: ShareRole): void
}>()

const role = computed({
  get: () => props.role,
  set: (value: ShareRole) => emit('update:role', value)
})
</script>

<template>
  <div
    class="border border-brownish-200 dark:border-brownish-700 rounded-lg p-3 space-y-3"
    data-testid="share-dialog-group-panel"
  >
    <div class="flex items-baseline justify-between gap-2 min-w-0">
      <span class="text-sm font-medium truncate">{{ group.name }}</span>
      <span
        v-if="group.memberCount !== null"
        class="text-xs text-brownish-300 shrink-0"
        data-testid="share-dialog-group-member-count"
      >
        {{ group.memberCount }} member{{ group.memberCount === 1 ? '' : 's' }}
      </span>
    </div>
    <p class="text-xs text-brownish-300" data-testid="share-dialog-group-note">
      Every member of this group receives the share at the role you pick.
    </p>
    <div>
      <span class="block text-xs uppercase tracking-wider text-brownish-300 mb-1.5">Access</span>
      <SharingPeopleAddRoleChips
        v-model="role"
        testid-prefix="share-dialog-group-role"
        :disabled="submitting || readOnly"
        :disable-co-owner="disableCoOwner"
      />
    </div>
  </div>
</template>
