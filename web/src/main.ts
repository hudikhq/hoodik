import { createApp } from 'vue'
import App from './App.vue'
import router from './router'
import { createPinia } from '!/init'
import { store as style } from '!/style'
import { lightModeKey, styleKey } from '@/config'
import { greeting } from '!/logger'
import { i18n, currentLocale } from '@/i18n'
import Notifications, { notify } from '@kyvg/vue3-notification'
import './css/main.css'

greeting()

window.addEventListener('unhandledrejection', function (event) {
  notify({
    title: event.reason.message || i18n.global.t('errors.unknown'),
    text: event.reason.description,
    type: 'error'
  })
})

const pinia = createPinia()

document.documentElement.setAttribute('lang', currentLocale())

/* Create Vue app */
createApp(App).use(i18n).use(Notifications).use(router).use(pinia).mount('#app')

/* Init Pinia stores */
const styleStore = style(pinia)

/* App style */
styleStore.setStyle(localStorage[styleKey] ?? 'basic')

/* Dark mode */
if (
  (!localStorage[lightModeKey] && window.matchMedia('(prefers-color-scheme: dark)').matches) ||
  localStorage[lightModeKey] === '1'
) {
  styleStore.setDarkMode(true)
}

/* Default title tag */
window.defaultDocumentTitle = import.meta.env.APP_NAME || 'Hoodik'

/* Set document title from route meta */
router.afterEach((to) => {
  document.title = to.meta?.title
    ? `${i18n.global.t(to.meta.title as string)} — ${window.defaultDocumentTitle}`
    : window.defaultDocumentTitle
})
