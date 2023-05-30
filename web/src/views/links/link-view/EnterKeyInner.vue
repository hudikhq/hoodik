<script setup lang="ts">
import LayoutGuest from '@/layouts/LayoutGuest.vue'
import SectionFullScreen from '@/components/ui/SectionFullScreen.vue'
import CardBox from '@/components/ui/CardBox.vue'
import { AppForm, AppField, AppButton } from '@/components/form'
import * as yup from 'yup'

defineProps<{
  unlockingError?: boolean
}>()
const emits = defineEmits<{
  (event: 'unlock', password: string): void
}>()

const config = {
  initialValues: {
    linkKeyHex: ''
  },
  validationSchema: yup.object().shape({
    linkKeyHex: yup.string().required('Link decryption key is required')
  }),
  onSubmit: async (values: { linkKeyHex: string }) => {
    emits('unlock', values.linkKeyHex)
  }
}
</script>
<template>
  <LayoutGuest>
    <SectionFullScreen v-slot="{ cardClass }" bg="pinkRed">
      <CardBox :class="cardClass" v-if="config">
        <h1 class="text-2xl text-white mb-5">Unlock The Link</h1>
        <p>
          You have received a link to a file that is locked, the link didn't contain the unlocking
          key, so please enter it below in order to view and download the link content.
        </p>

        <AppForm :config="config" class="mt-8 space-y-6" v-slot="{ form }">
          <AppField
            type="password"
            :form="form"
            label="Link key (password)"
            name="linkKeyHex"
            placeholder="********"
            :autofocus="true"
          />

          <p v-if="unlockingError" class="text-sm text-redish-400">
            {{ unlockingError }}
          </p>

          <AppButton color="info" :form="form" type="submit">Unlock</AppButton>
        </AppForm>
      </CardBox>
    </SectionFullScreen>
  </LayoutGuest>
</template>
