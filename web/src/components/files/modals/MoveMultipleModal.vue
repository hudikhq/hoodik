<script setup lang="ts">
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import MoveIntoSharedConfirmModal from '@/components/files/modals/MoveIntoSharedConfirmModal.vue'
import type { AppFile, Authenticated, FilesStore, KeyPair } from 'types'
import DirectoryTree from '@/components/files/browser/DirectoryTree.vue'
import { useRouter } from 'vue-router'
import { ref } from 'vue'
import { classifyMove, executeMove } from '!/storage/moveInto'
import type { MoveCascadePreview } from '!/shares'
import { useCapability } from '@/composables/useCapability'
import { errorNotification } from '!/index'

const router = useRouter()
const { sharingEnabled } = useCapability()

const props = defineProps<{
  modelValue: boolean
  Storage: FilesStore
  kp: KeyPair
  authenticated: Authenticated
}>()

const emits = defineEmits<{
  (event: 'update:modelValue', value: boolean): void
}>()

interface ConfirmState {
  visible: boolean
  folderName: string
  destinationName: string
  itemCount: number
  memberLabels: string[]
  progress: number | null
  resolve: ((accepted: boolean) => void) | null
}

const confirm = ref<ConfirmState>({
  visible: false,
  folderName: '',
  destinationName: '',
  itemCount: 0,
  memberLabels: [],
  progress: null,
  resolve: null
})

function memberLabels(preview: MoveCascadePreview): string[] {
  return preview.members
    .filter((m) => m.user_id !== props.authenticated.user.id)
    .map((m) => m.email ?? m.user_id)
}

function confirmCascade(
  folder: AppFile,
  destination: AppFile | undefined,
  preview: MoveCascadePreview
): Promise<boolean> {
  return new Promise((resolve) => {
    confirm.value = {
      visible: true,
      folderName: folder.name,
      destinationName: destination?.name ?? 'Home',
      itemCount: preview.itemCount,
      memberLabels: memberLabels(preview),
      progress: null,
      resolve
    }
  })
}

function resolveConfirm(accepted: boolean): void {
  const resolve = confirm.value.resolve
  confirm.value = { ...confirm.value, visible: false, resolve: null }
  resolve?.(accepted)
}

const select = async (file?: AppFile) => {
  if (!props.Storage.selected) {
    return
  }

  const decision = classifyMove({
    sources: props.Storage.selected,
    destination: file,
    sourceParent: props.Storage.dir,
    sharingEnabled: sharingEnabled.value
  })

  if (decision.kind === 'blocked') {
    errorNotification(decision.message)
    return
  }

  try {
    await executeMove(decision, {
      callerUserId: props.authenticated.user.id,
      keypair: props.kp,
      plainMove: (sources, destinationId) =>
        props.Storage.moveAll(props.kp, sources, destinationId),
      confirmCascade: (folder, preview) => confirmCascade(folder, file, preview),
      onProgress: (p) => {
        if (confirm.value.visible) {
          confirm.value = {
            ...confirm.value,
            progress: p.totalKeys > 0 ? p.wrappedKeys / p.totalKeys : null
          }
        }
      }
    })
  } catch (error) {
    errorNotification(error)
    return
  }

  // The plain path already refreshes the listing inside `Storage.moveAll`;
  // the shared paths re-parent server-side, so refresh the source directory
  // to drop the moved rows.
  if (decision.kind !== 'plain') {
    await props.Storage.find(props.kp, props.Storage.dir?.id, true)
  }

  emits('update:modelValue', false)
  router.push({ name: 'files', params: { file_id: file?.id } })
}
</script>

<template>
  <CardBoxModal
    title="Select target directory"
    :model-value="props.modelValue"
    :has-cancel="false"
    :has-close="true"
    :hide-submit="true"
    @cancel="emits('update:modelValue', false)"
  >
    <div class="w-full border-[1px] border-t-0 border-brownish-800 min-h-[400px]">
      <DirectoryTree
        v-if="props.modelValue"
        @select="select"
        :keypair="props.kp"
        :Storage="props.Storage"
        load
      />
    </div>
  </CardBoxModal>

  <MoveIntoSharedConfirmModal
    v-model="confirm.visible"
    :folder-name="confirm.folderName"
    :destination-name="confirm.destinationName"
    :item-count="confirm.itemCount"
    :member-labels="confirm.memberLabels"
    :progress="confirm.progress"
    @confirm="resolveConfirm(true)"
    @cancel="resolveConfirm(false)"
  />
</template>
