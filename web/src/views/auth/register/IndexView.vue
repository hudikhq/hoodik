<script setup lang="ts">
import { AppForm, AppField, AppButton } from '@/components/form'
import * as yup from 'yup'
import { isStrongPassword } from '@/utils/password'
import { store } from '!/auth/register'
import { useRoute, useRouter } from 'vue-router'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import LayoutGuest from '@/layouts/LayoutGuest.vue'
import SectionFullScreen from '@/components/ui/SectionFullScreen.vue'
import CardBox from '@/components/ui/CardBox.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import type { CreateUser } from 'types'
import * as logger from '!/logger'

const register = store()
const router = useRouter()
const route = useRoute()
const { t } = useI18n()

const config = ref()
const working = ref(false)

register.getStatus()

// Invited users (with an `invitation_id` query param) bypass the public
// allow_register flag server-side, so we keep the form available for them
// even when general registration is closed.
const hasInvitation = computed(() => !!route.query.invitation_id)
const registrationDisabled = computed(
  () => register.allowRegister === false && !hasInvitation.value
)

const init = () => {
  register.preload(route)

  const initialValues = register.createUser
  const initialErrors = register.errors || {}

  config.value = {
    initialValues: initialValues,
    initialErrors,
    validationSchema: yup.object().shape({
      email: yup
        .string()
        .required(t('auth.validation.emailRequired'))
        .email(t('auth.validation.emailInvalid')),
      password: yup
        .string()
        .required(t('auth.validation.passwordRequired'))
        .test(
          'weak-password',
          t('auth.validation.passwordTooWeak'),
          (value: string | undefined) => isStrongPassword(value)
        ),
      confirm_password: yup
        .string()
        .required(t('auth.validation.confirmPasswordRequired'))
        .oneOf([yup.ref('password')], t('auth.validation.passwordsDoNotMatch'))
    }),
    onSubmit: async (values: Partial<CreateUser>) => {
      logger.debug(values)
      register.set(values)

      router.push({ name: 'register-key' })
      working.value = true
    }
  }
}

init()
</script>
<template>
  <LayoutGuest>
    <SectionFullScreen v-slot="{ cardClass }" bg="pinkRed">
      <CardBox :class="cardClass">
        <h1 class="text-2xl text-brownish-700 dark:text-white">{{ $t('auth.register.title') }}</h1>

        <div v-if="registrationDisabled" class="mt-8 space-y-6" data-testid="registration-disabled">
          <p class="text-sm text-dirty-white">
            {{ $t('auth.register.disabled') }}
          </p>
          <p class="text-sm text-brownish-300">
            {{ $t('auth.register.useInvitation') }}
          </p>
          <BaseButton
            :to="{ name: 'login' }"
            color="info"
            :label="$t('auth.register.backToLogin')"
          />
        </div>

        <AppForm
          v-else-if="config"
          :config="config"
          :working="working"
          class="mt-8 space-y-6"
          v-slot="{ form }"
        >
          <AppField
            :form="form"
            :label="$t('auth.yourEmail')"
            name="email"
            :placeholder="$t('auth.emailPlaceholder')"
            autocomplete="username"
            :disabled="form.values.invitation_id"
          />
          <AppField
            type="password"
            :form="form"
            :label="$t('auth.yourPassword')"
            name="password"
            autocomplete="new-password"
            placeholder="••••••••"
          />
          <AppField
            :allow-copy="false"
            type="password"
            :form="form"
            :label="$t('auth.register.confirmPassword')"
            name="confirm_password"
            autocomplete="new-password"
            placeholder="••••••••"
          />
          <AppButton color="info" type="submit">{{ $t('common.next') }}</AppButton>

          <div class="text-sm font-medium text-brownish-500 dark:text-brownish-100">
            {{ $t('auth.alreadyHaveAccount') }}
            <router-link
              :to="{ name: 'login' }"
              class="text-primary-700 hover:underline dark:text-primary-500"
              >{{ $t('common.login') }}</router-link
            >
          </div>
        </AppForm>
      </CardBox>
    </SectionFullScreen>
  </LayoutGuest>
</template>
