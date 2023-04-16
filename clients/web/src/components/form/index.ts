import AppButton from './AppButton.vue'
import AppCheckbox from './AppCheckbox.vue'
import AppField from './AppField.vue'
import AppForm from './AppForm.vue'
import AppBrowse from './AppBrowse.vue'

export { AppButton, AppCheckbox, AppField, AppForm, AppBrowse }

import type { useForm } from 'vee-validate'
export type FormType = ReturnType<typeof useForm>
