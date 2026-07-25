import { createI18n } from 'vue-i18n'
import { enUS, fr as frFR, de as deDE, hr as hrHR } from 'date-fns/locale'
import type { Locale as DateFnsLocale } from 'date-fns'
import { localeKey } from '@/config'
import en from '@/locales/en.json'
import fr from '@/locales/fr.json'
import de from '@/locales/de.json'
import hr from '@/locales/hr.json'

/**
 * Native-language labels, shown in the language picker exactly as written
 * regardless of the active locale.
 */
export const SUPPORTED_LOCALES = {
  en: 'English',
  fr: 'Français',
  de: 'Deutsch',
  hr: 'Hrvatski'
} as const

export type SupportedLocale = keyof typeof SUPPORTED_LOCALES

const dateFnsLocales: Record<SupportedLocale, DateFnsLocale> = {
  en: enUS,
  fr: frFR,
  de: deDE,
  hr: hrHR
}

function isSupported(locale: string): locale is SupportedLocale {
  return locale in SUPPORTED_LOCALES
}

/**
 * Stored preference wins, then the browser language, then English.
 */
export function detectLocale(): SupportedLocale {
  const stored = localStorage[localeKey]

  if (typeof stored === 'string' && isSupported(stored)) {
    return stored
  }

  const browser = (navigator.language || 'en').slice(0, 2).toLowerCase()

  return isSupported(browser) ? browser : 'en'
}

/**
 * Croatian needs three plural forms (1 → singular, 2-4 → paucal, else →
 * plural, with 11-14 always plural); vue-i18n's default resolver only
 * understands the English one/other split.
 */
function croatianPluralRule(choice: number, choicesLength: number): number {
  if (choicesLength < 3) {
    return choice === 1 ? 0 : 1
  }

  const mod10 = choice % 10
  const mod100 = choice % 100

  if (mod10 === 1 && mod100 !== 11) return 0
  if (mod10 >= 2 && mod10 <= 4 && (mod100 < 12 || mod100 > 14)) return 1
  return 2
}

export const i18n = createI18n({
  legacy: false,
  globalInjection: true,
  locale: detectLocale(),
  fallbackLocale: 'en',
  pluralRules: { hr: croatianPluralRule },
  messages: { en, fr, de, hr }
})

export function currentLocale(): SupportedLocale {
  return i18n.global.locale.value as SupportedLocale
}

export function setLocale(locale: SupportedLocale) {
  i18n.global.locale.value = locale
  localStorage[localeKey] = locale
  document.documentElement.setAttribute('lang', locale)
}

/**
 * date-fns needs its own locale object for month names and relative phrases.
 */
export function currentDateFnsLocale(): DateFnsLocale {
  return dateFnsLocales[currentLocale()]
}

/**
 * The backend responds with snake_case error codes (never prose) so each
 * client can render them in its own language. Codes may carry a detail
 * suffix after a colon (`invalid_id_provided_while_extracting:user`).
 * Unknown codes render as-is rather than hiding behind a generic message.
 */
export function translateErrorCode(code?: string | null): string {
  if (!code) {
    return i18n.global.t('errors.unknown')
  }

  const [base, ...detail] = code.split(':')
  const key = `errors.${base}`

  if (!i18n.global.te(key)) {
    return code
  }

  const message = i18n.global.t(key)

  return detail.length ? `${message} (${detail.join(':')})` : message
}
