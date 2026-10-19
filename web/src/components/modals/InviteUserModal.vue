<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { mdiCheck, mdiContentCopy } from '@mdi/js'
import QRCodeComponent from 'qrcode.vue'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import FormError from '@/components/ui/FormError.vue'
import { AppForm, AppField } from '@/components/form'
import QuotaInput from '@/components/ui/QuotaInput.vue'
import * as yup from 'yup'
import type { ErrorResponse } from '!/api'
import { create } from '!/admin/invitations'
import type { Create, CreatedInvitation } from 'types/admin/invitations'
import { useI18n } from 'vue-i18n'
import { formatPrettyDate, humanizeError } from '!/index'

const props = defineProps<{
  modelValue?: boolean | undefined
}>()

const { t } = useI18n()
const router = useRouter()

const emit = defineEmits(['update:modelValue', 'cancel', 'confirm'])

const config = ref()
const errorMessage = ref()

/** Set when the server couldn't email the invitation, so the admin shares it by hand. */
const unsent = ref<{ invitation: CreatedInvitation; link: string }>()
const copied = ref(false)

function registerLink(invitation: CreatedInvitation): string {
  const { href } = router.resolve({
    name: 'register',
    query: { invitation_id: invitation.id, email: invitation.email }
  })
  return new URL(href, window.location.origin).toString()
}

async function copyLink() {
  if (!unsent.value) return
  await navigator.clipboard.writeText(unsent.value.link)
  copied.value = true
  setTimeout(() => (copied.value = false), 2000)
}

function closeUnsent() {
  unsent.value = undefined
  copied.value = false
  emit('update:modelValue', false)
}

const init = () => {
  config.value = {
    initialValues: {
      email: '',
      message: '',
      quota: undefined,
      role: undefined
    } as Create,
    validationSchema: yup.object().shape({
      email: yup.string().email().required(t('account.invite.emailRequired')),
      quota: yup.number().min(0)
    }),
    onSubmit: async (values: Create, ctx: any) => {
      try {
        const invitation = await create(values)
        ctx.resetForm()
        emit('confirm')

        if (invitation.email_sent) {
          emit('update:modelValue', false)
        } else {
          unsent.value = { invitation, link: registerLink(invitation) }
        }
      } catch (err) {
        const error = err as ErrorResponse<unknown>
        config.value.initialErrors = error.validation || {}
        errorMessage.value = humanizeError(err)
      }
    }
  }
}

init()
</script>

<template>
  <CardBoxModal
    v-if="unsent"
    :modelValue="props.modelValue"
    @update:modelValue="closeUnsent"
    :title="$t('account.invite.linkTitle')"
    button="info"
    :buttonLabel="$t('common.done')"
    has-close
    @confirm="closeUnsent"
    @cancel="closeUnsent"
    data-testid="invite-link-modal"
  >
    <div class="space-y-4 text-sm">
      <p>{{ $t('account.invite.noEmailBody', { email: unsent.invitation.email }) }}</p>

      <div class="flex flex-col sm:flex-row gap-5 items-center sm:items-start">
        <!-- Fixed white plate in both themes: a dark-inverted QR code scans
             badly on most phone cameras. -->
        <div class="shrink-0 rounded-lg bg-white p-3" data-testid="invite-link-qr">
          <QRCodeComponent :value="unsent.link" :size="150" render-as="svg" :margin="0" level="M" />
        </div>

        <div class="space-y-3 min-w-0 w-full">
          <div class="flex items-stretch gap-2">
            <input
              type="text"
              readonly
              :value="unsent.link"
              @focus="($event.target as HTMLInputElement).select()"
              data-testid="invite-link-url"
              class="min-w-0 flex-1 px-3 py-1.5 text-xs font-mono rounded-lg bg-white dark:bg-brownish-800 border border-paper-300 dark:border-brownish-600 text-brownish-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-redish-400/60"
            />
            <button
              type="button"
              @click="copyLink"
              :title="copied ? $t('common.copied') : $t('common.copy')"
              :aria-label="copied ? $t('common.copied') : $t('common.copy')"
              class="shrink-0 px-2.5 rounded-lg border border-paper-300 dark:border-brownish-600 text-brownish-400 dark:text-brownish-50 hover:text-brownish-900 dark:hover:text-white transition-colors"
            >
              <BaseIcon :path="copied ? mdiCheck : mdiContentCopy" :size="16" />
            </button>
          </div>
          <p class="text-brownish-600 dark:text-dirty-white/70">
            {{ $t('account.invite.linkWarning', { date: formatPrettyDate(unsent.invitation.expires_at) }) }}
          </p>
        </div>
      </div>
    </div>
  </CardBoxModal>

  <AppForm v-else-if="config" :config="config" v-slot="{ form }">
    <CardBoxModal
      :modelValue="props.modelValue"
      @update:modelValue="$emit('update:modelValue', $event)"
      :title="$t('account.invite.title')"
      button="info"
      :buttonLabel="$t('account.invite.submit')"
      has-cancel
      @cancel="$emit('cancel')"
      :form="form"
    >
      <FormError v-if="errorMessage">{{ errorMessage }}</FormError>

      <AppField :form="form" :label="$t('common.email')" name="email" autofocus />
      <AppField
        :form="form"
        :label="$t('account.invite.messageLabel')"
        name="message"
        :textarea="true"
      />

      <QuotaInput
        :model-value="form.values.quota"
        @update:model-value="(v) => form.setValues({ quota: v })"
        :title="$t('account.invite.quotaTitle')"
      />
    </CardBoxModal>
  </AppForm>
</template>
