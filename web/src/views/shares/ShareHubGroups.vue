<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import {
  mdiAccountMultipleOutline,
  mdiAccountPlus,
  mdiAccountRemoveOutline,
  mdiDeleteOutline,
  mdiPencilOutline,
  mdiPlus
} from '@mdi/js'

import BaseButton from '@/components/ui/BaseButton.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import { AppField } from '@/components/form'
import GroupCreateDialog from '@/components/shares/groups/GroupCreateDialog.vue'
import GroupAddMemberDialog from '@/components/shares/groups/GroupAddMemberDialog.vue'
import { useCapability } from '@/composables/useCapability'

import { api as sharesApi, crypto as shareCrypto } from '!/shares'
import { errorNotification, notification } from '!/index'

import type {
  AppShareGroupAsMember,
  AppShareGroupWithMembers,
  GroupMemberWithKey,
  GroupRole,
  KeyPair
} from 'types'

const props = defineProps<{
  authenticated?: { user: { id: string; email: string } } | null
  keypair?: KeyPair
}>()

const { t } = useI18n()
const { shareGroups } = useCapability()

const loading = ref(false)
const owned = ref<AppShareGroupWithMembers[]>([])
const memberOf = ref<AppShareGroupAsMember[]>([])

/** Lazily-loaded rosters for member-of groups the caller co-owns — the
 *  list endpoint only ships the caller's own role for those, so a co-owner
 *  fetches the peer roster on demand to manage it. The roster includes the
 *  group owner (carried with `group_role: "owner"`); the manage UI excludes
 *  that row from per-member role/remove controls. */
const managedRosters = ref<Record<string, GroupMemberWithKey[]>>({})

const showCreate = ref(false)
const addingTo = ref<{ id: string; name: string; canGrantCoOwner: boolean } | null>(null)
const renaming = ref<{ id: string; name: string } | null>(null)
const renameValue = ref('')

const deletingGroup = ref<AppShareGroupWithMembers | null>(null)
const removingMember = ref<{
  groupId: string
  groupName: string
  userId: string
  email: string | null
} | null>(null)

onMounted(refresh)

async function refresh(): Promise<void> {
  loading.value = true
  try {
    const response = await sharesApi.listGroups()
    owned.value = response.owned
    memberOf.value = response.member_of
    // Refresh any rosters we'd already loaded so a re-render after a
    // mutation reflects the new state without a second click.
    for (const groupId of Object.keys(managedRosters.value)) {
      await loadManagedRoster(groupId)
    }
  } catch (err) {
    errorNotification(err)
  } finally {
    loading.value = false
  }
}

async function loadManagedRoster(groupId: string): Promise<void> {
  try {
    managedRosters.value = {
      ...managedRosters.value,
      [groupId]: await sharesApi.groupMembers(groupId)
    }
  } catch (err) {
    errorNotification(err)
  }
}

/** Drop the owner row from a fetched roster — the owner can't be a managed
 *  member (no role select, no remove control). */
function manageableMembers(groupId: string): GroupMemberWithKey[] {
  return (managedRosters.value[groupId] ?? []).filter((m) => m.group_role !== 'owner')
}

function deleteGroup(group: AppShareGroupWithMembers): void {
  deletingGroup.value = group
}

async function confirmDeleteGroup(): Promise<void> {
  const group = deletingGroup.value
  deletingGroup.value = null
  if (!group) return
  try {
    await sharesApi.deleteGroup(group.id)
    notification(t('shares.groups.deletedTitle'), t('shares.groups.deletedBody', { name: group.name }), 'success')
    await refresh()
  } catch (err) {
    errorNotification(err)
  }
}

function removeMember(
  groupId: string,
  groupName: string,
  member: { user_id: string; email: string | null }
): void {
  removingMember.value = {
    groupId,
    groupName,
    userId: member.user_id,
    email: member.email
  }
}

async function confirmRemoveMember(): Promise<void> {
  const target = removingMember.value
  removingMember.value = null
  if (!target) return
  try {
    await sharesApi.removeGroupMember(target.groupId, target.userId)
    notification(
      t('shares.groups.memberRemovedTitle'),
      t('shares.groups.memberRemovedBody'),
      'success'
    )
    await refresh()
  } catch (err) {
    errorNotification(err)
  }
}

function openAddMember(groupId: string, groupName: string, canGrantCoOwner: boolean): void {
  addingTo.value = { id: groupId, name: groupName, canGrantCoOwner }
}

async function onMemberAdded(): Promise<void> {
  await refresh()
}

function startRename(groupId: string, currentName: string): void {
  renaming.value = { id: groupId, name: currentName }
  renameValue.value = currentName
}

async function confirmRename(): Promise<void> {
  const target = renaming.value
  if (!target) return
  const trimmed = renameValue.value.trim()
  if (!trimmed || trimmed === target.name) {
    renaming.value = null
    return
  }
  renaming.value = null
  try {
    await sharesApi.renameGroup(target.id, trimmed)
    notification(t('shares.groups.renamedTitle'), t('shares.groups.renamedBody', { name: trimmed }), 'success')
    await refresh()
  } catch (err) {
    const message = err instanceof Error ? err.message : ''
    if (/409|conflict|taken/i.test(message)) {
      notification(t('shares.groups.nameTakenTitle'), t('shares.groups.nameTakenBody'), 'error')
    } else {
      errorNotification(err)
    }
  }
}

const ALL_GROUP_ROLES: GroupRole[] = ['reader', 'editor', 'co-owner']

/** Roles a co-owner manager may set a member to: reader/editor only,
 *  never co-owner — fail-closed to match the server's escalation guard.
 *  The group owner is unconstrained and uses {@link ALL_GROUP_ROLES}. */
const CO_OWNER_SETTABLE_ROLES: GroupRole[] = ['reader', 'editor']

async function changeRole(
  groupId: string,
  userId: string,
  next: GroupRole
): Promise<void> {
  try {
    await sharesApi.setGroupMemberRole(groupId, userId, next)
    notification(t('shares.groups.roleUpdatedTitle'), t('shares.groups.roleUpdatedBody', { role: next }), 'success')
    await refresh()
  } catch (err) {
    errorNotification(err)
  }
}

function shortFingerprint(fp: string): string {
  return shareCrypto.formatFingerprint(fp).split('-').slice(0, 2).join('-')
}

function roleLabel(role: GroupRole): string {
  return role === 'co-owner'
    ? t('shares.roles.coOwner')
    : role === 'editor'
      ? t('shares.roles.editor')
      : t('shares.roles.reader')
}

const ownedHasGroups = computed(() => owned.value.length > 0)
const memberOfHasGroups = computed(() => memberOf.value.length > 0)
const senderId = computed(() => props.authenticated?.user.id ?? '')

const addDialogCanGrantCoOwner = computed(() => addingTo.value?.canGrantCoOwner ?? false)
</script>

<template>
  <div data-testid="share-hub-groups">
    <header class="flex items-center justify-between gap-3 mb-3">
      <h2 class="text-xs font-semibold uppercase tracking-wider text-brownish-300">{{ $t('shares.groups.owned') }}</h2>
      <BaseButton
        :icon="mdiPlus"
        color="info"
        small
        :label="$t('shares.groups.new')"
        data-testid="share-hub-groups-new"
        @click.prevent="showCreate = true"
      />
    </header>

    <p v-if="loading" class="text-sm text-brownish-300" data-testid="share-hub-groups-loading">
      {{ $t('shares.groups.loading') }}
    </p>

    <ul v-if="ownedHasGroups" class="space-y-3" data-testid="share-hub-groups-owned-list">
      <li
        v-for="group in owned"
        :key="group.id"
        class="p-3 sm:p-4 rounded-lg bg-brownish-50 dark:bg-brownish-900/60 border border-brownish-200 dark:border-brownish-700"
        :data-testid="`share-hub-groups-owned-${group.id}`"
      >
        <header class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2">
          <div class="flex items-center gap-2 min-w-0">
            <BaseIcon :path="mdiAccountMultipleOutline" :size="20" class="shrink-0 text-brownish-300 dark:text-brownish-200" />
            <span
              class="text-sm font-medium truncate"
              :data-testid="`share-hub-groups-owned-${group.id}-name`"
            >
              {{ group.name }}
            </span>
            <span class="text-xs text-brownish-300 shrink-0">
              · {{ $t('shares.groups.memberCount', group.members.length) }}
            </span>
          </div>
          <div class="flex gap-1.5 shrink-0">
            <BaseButton
              v-if="shareGroups"
              :icon="mdiPencilOutline"
              color="dark"
              small
              :label="$t('common.rename')"
              :data-testid="`share-hub-groups-owned-${group.id}-rename`"
              @click.prevent="startRename(group.id, group.name)"
            />
            <BaseButton
              :icon="mdiAccountPlus"
              color="dark"
              small
              :label="$t('common.add')"
              :data-testid="`share-hub-groups-owned-${group.id}-add`"
              @click.prevent="openAddMember(group.id, group.name, group.owner_id === senderId)"
            />
            <BaseButton
              :icon="mdiDeleteOutline"
              color="danger"
              small
              :label="$t('common.delete')"
              :data-testid="`share-hub-groups-owned-${group.id}-delete`"
              @click.prevent="deleteGroup(group)"
            />
          </div>
        </header>

        <ul
          v-if="group.members.length > 0"
          class="mt-3 space-y-1.5"
          :data-testid="`share-hub-groups-owned-${group.id}-members`"
        >
          <li
            v-for="member in group.members"
            :key="member.user_id"
            class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2 text-sm bg-white dark:bg-brownish-800/60 rounded-lg px-3 py-2"
            :data-testid="`share-hub-groups-owned-${group.id}-member-${member.user_id}`"
          >
            <div class="min-w-0">
              <div class="truncate">{{ member.email }}</div>
              <span class="font-mono text-xs text-brownish-400">
                {{ shortFingerprint(member.fingerprint) }}
              </span>
            </div>
            <div class="flex items-center justify-end gap-2 shrink-0">
              <select
                v-if="shareGroups"
                class="text-xs rounded-md border border-brownish-200 dark:border-brownish-700 bg-white dark:bg-brownish-800 px-1.5 py-1"
                :value="member.group_role"
                :data-testid="`share-hub-groups-owned-${group.id}-member-${member.user_id}-role`"
                @change="(e) => changeRole(group.id, member.user_id, (e.target as HTMLSelectElement).value as GroupRole)"
              >
                <option v-for="r in ALL_GROUP_ROLES" :key="r" :value="r">
                  {{ roleLabel(r) }}
                </option>
              </select>
              <span
                v-else
                class="text-xs text-brownish-400"
                :data-testid="`share-hub-groups-owned-${group.id}-member-${member.user_id}-role-label`"
              >
                {{ roleLabel(member.group_role) }}
              </span>
              <BaseButton
                :icon="mdiAccountRemoveOutline"
                color="danger"
                small
                rounded-full
                :title="$t('shares.groups.removeMember')"
                :data-testid="`share-hub-groups-owned-${group.id}-member-${member.user_id}-remove`"
                @click.prevent="removeMember(group.id, group.name, member)"
              />
            </div>
          </li>
        </ul>
        <p
          v-else
          class="mt-3 text-xs text-brownish-300"
          :data-testid="`share-hub-groups-owned-${group.id}-empty`"
        >
          {{ $t('shares.groups.noMembers') }}
        </p>
      </li>
    </ul>

    <p
      v-else-if="!loading"
      class="text-sm text-brownish-300 p-4 rounded-lg bg-brownish-50 dark:bg-brownish-900/60 border border-brownish-200 dark:border-brownish-700"
      data-testid="share-hub-groups-owned-empty"
    >
      {{ $t('shares.groups.ownedEmpty') }}
    </p>

    <header class="flex items-center justify-between mt-6 mb-3">
      <h2 class="text-xs font-semibold uppercase tracking-wider text-brownish-300">{{ $t('shares.groups.memberOf') }}</h2>
    </header>
    <ul
      v-if="memberOfHasGroups"
      class="space-y-2"
      data-testid="share-hub-groups-member-of-list"
    >
      <li
        v-for="group in memberOf"
        :key="group.id"
        class="p-3 rounded-lg bg-brownish-50 dark:bg-brownish-900/60 border border-brownish-200 dark:border-brownish-700 text-sm"
        :data-testid="`share-hub-groups-member-of-${group.id}`"
      >
        <div class="flex flex-wrap items-baseline justify-between gap-x-2 gap-y-1">
          <div class="flex flex-wrap items-baseline gap-x-2 min-w-0">
            <span class="font-medium truncate">{{ group.name }}</span>
            <span class="text-xs text-brownish-300">{{ $t('shares.groups.ownedBy', { email: group.owner_email }) }}</span>
          </div>
          <span
            v-if="shareGroups"
            class="text-xs px-2 py-0.5 rounded-full bg-brownish-100 dark:bg-brownish-800 text-brownish-700 dark:text-brownish-200 shrink-0"
            :data-testid="`share-hub-groups-member-of-${group.id}-role`"
          >
            {{ $t('shares.groups.yourRole', { role: roleLabel(group.group_role) }) }}
          </span>
        </div>

        <template v-if="shareGroups">
          <p
            v-if="group.group_role === 'editor'"
            class="mt-2 text-xs text-brownish-300"
            :data-testid="`share-hub-groups-member-of-${group.id}-editor-hint`"
          >
            {{ $t('shares.groups.editorHint') }}
          </p>

          <div
            v-else-if="group.group_role === 'co-owner'"
            class="mt-3"
            :data-testid="`share-hub-groups-member-of-${group.id}-manage`"
          >
            <div class="flex gap-1.5 flex-wrap">
              <BaseButton
                :icon="mdiAccountPlus"
                color="dark"
                small
                :label="$t('shares.groups.addMember')"
                :data-testid="`share-hub-groups-member-of-${group.id}-add`"
                @click.prevent="openAddMember(group.id, group.name, false)"
              />
              <BaseButton
                v-if="!managedRosters[group.id]"
                color="info"
                outline
                small
                :label="$t('shares.groups.manageRoster')"
                :data-testid="`share-hub-groups-member-of-${group.id}-load-roster`"
                @click.prevent="loadManagedRoster(group.id)"
              />
            </div>

            <ul
              v-if="managedRosters[group.id]"
              class="mt-3 space-y-1.5"
              :data-testid="`share-hub-groups-member-of-${group.id}-members`"
            >
              <li
                v-for="member in manageableMembers(group.id)"
                :key="member.user_id"
                class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2 bg-white dark:bg-brownish-800/60 rounded-lg px-3 py-2"
                :data-testid="`share-hub-groups-member-of-${group.id}-member-${member.user_id}`"
              >
                <div class="min-w-0">
                  <div class="truncate">{{ member.email }}</div>
                  <span class="font-mono text-xs text-brownish-400">
                    {{ shortFingerprint(member.fingerprint) }}
                  </span>
                </div>
                <div class="flex items-center justify-end gap-2 shrink-0">
                  <select
                    v-if="member.group_role !== 'co-owner'"
                    class="text-xs rounded-md border border-brownish-200 dark:border-brownish-700 bg-white dark:bg-brownish-800 px-1.5 py-1"
                    :value="member.group_role"
                    :data-testid="`share-hub-groups-member-of-${group.id}-member-${member.user_id}-role`"
                    @change="(e) => changeRole(group.id, member.user_id, (e.target as HTMLSelectElement).value as GroupRole)"
                  >
                    <option v-for="r in CO_OWNER_SETTABLE_ROLES" :key="r" :value="r">
                      {{ roleLabel(r) }}
                    </option>
                  </select>
                  <span
                    v-else
                    class="text-xs text-brownish-400"
                    :data-testid="`share-hub-groups-member-of-${group.id}-member-${member.user_id}-role-label`"
                  >
                    {{ roleLabel(member.group_role) }}
                  </span>
                  <BaseButton
                    v-if="member.group_role !== 'co-owner'"
                    :icon="mdiAccountRemoveOutline"
                    color="danger"
                    small
                    rounded-full
                    :title="$t('shares.groups.removeMember')"
                    :data-testid="`share-hub-groups-member-of-${group.id}-member-${member.user_id}-remove`"
                    @click.prevent="removeMember(group.id, group.name, member)"
                  />
                </div>
              </li>
            </ul>
          </div>
        </template>
      </li>
    </ul>
    <p
      v-else-if="!loading"
      class="text-sm text-brownish-300 p-4 rounded-lg bg-brownish-50 dark:bg-brownish-900/60 border border-brownish-200 dark:border-brownish-700"
      data-testid="share-hub-groups-member-of-empty"
    >
      {{ $t('shares.groups.memberOfEmpty') }}
    </p>

    <GroupCreateDialog
      v-if="showCreate"
      v-model="showCreate"
      data-testid="share-hub-groups-create-dialog"
      @created="() => refresh()"
      @cancel="showCreate = false"
    />

    <GroupAddMemberDialog
      v-if="addingTo"
      :model-value="!!addingTo"
      :group-id="addingTo.id"
      :group-name="addingTo.name"
      :can-grant-co-owner="addDialogCanGrantCoOwner"
      data-testid="share-hub-groups-add-dialog"
      @update:model-value="(value) => { if (!value) addingTo = null }"
      @added="onMemberAdded"
      @cancel="addingTo = null"
    />

    <CardBoxModal
      v-if="renaming"
      :model-value="true"
      :title="$t('shares.groups.renameTitle')"
      button="info"
      :button-label="$t('common.rename')"
      has-cancel
      data-testid="share-hub-groups-rename-modal"
      @confirm="confirmRename"
      @cancel="renaming = null"
    >
      <AppField
        name="group-rename"
        :label="$t('shares.groups.nameLabel')"
        v-model="renameValue"
        @confirm="confirmRename"
      />
    </CardBoxModal>

    <CardBoxModal
      v-if="deletingGroup"
      :model-value="true"
      :title="$t('shares.groups.deleteTitle')"
      button="danger"
      :button-label="$t('common.delete')"
      has-cancel
      @confirm="confirmDeleteGroup"
      @cancel="deletingGroup = null"
    >
      <div data-testid="share-hub-groups-delete-modal" class="space-y-2 text-sm">
        <i18n-t tag="p" keypath="shares.groups.deleteBody" scope="global">
          <template #name><strong>{{ deletingGroup.name }}</strong></template>
        </i18n-t>
      </div>
    </CardBoxModal>

    <CardBoxModal
      v-if="removingMember"
      :model-value="true"
      :title="$t('shares.groups.removeMember')"
      button="danger"
      :button-label="$t('common.remove')"
      has-cancel
      @confirm="confirmRemoveMember"
      @cancel="removingMember = null"
    >
      <div data-testid="share-hub-groups-remove-member-modal" class="space-y-2 text-sm">
        <i18n-t tag="p" keypath="shares.groups.removeMemberBody" scope="global">
          <template #member>
            <strong>{{ removingMember.email ?? $t('shares.groups.thisMember') }}</strong>
          </template>
          <template #group><strong>{{ removingMember.groupName }}</strong></template>
        </i18n-t>
      </div>
    </CardBoxModal>
  </div>
</template>
