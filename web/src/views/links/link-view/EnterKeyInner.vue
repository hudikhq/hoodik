<script setup lang="ts">
import LayoutGuest from '@/layouts/LayoutGuest.vue'
import SectionFullScreen from '@/components/ui/SectionFullScreen.vue'
import CardBox from '@/components/ui/CardBox.vue'
import { AppForm, AppField, AppButton } from '@/components/form'
import * as yup from 'yup'
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'

defineProps<{
  unlockingError?: string
}>()
const emits = defineEmits<{
  (event: 'unlock', password: string): void
}>()

const { t } = useI18n()

const config = ref({
  initialValues: {
    linkKeyHex: ''
  },
  validationSchema: yup.object().shape({
    linkKeyHex: yup.string().required(t('links.unlock.keyRequired'))
  }),
  onSubmit: async (values: { linkKeyHex: string }) => {
    emits('unlock', values.linkKeyHex)
  }
})
</script>
<template>
  <LayoutGuest>
    <SectionFullScreen v-slot="{ cardClass }" bg="pinkRed">
      <CardBox :class="cardClass" v-if="config">
        <h1 class="text-2xl text-white mb-5">{{ $t('links.unlock.title') }}</h1>
        <p>
          {{ $t('links.unlock.description') }}
        </p>

        <AppForm :config="config" class="mt-8 space-y-6" v-slot="{ form }">
          <AppField
            type="password"
            :form="form"
            :label="$t('links.unlock.keyLabel')"
            name="linkKeyHex"
            placeholder="********"
            :autofocus="true"
          />

          <p v-if="unlockingError" class="text-sm text-redish-400">
            {{ unlockingError }}
          </p>

          <AppButton color="info" :form="form" type="submit">{{ $t('links.unlock.submit') }}</AppButton>
        </AppForm>
      </CardBox>
    </SectionFullScreen>
  </LayoutGuest>
</template>
