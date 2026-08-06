import { mdiLock, mdiThemeLightDark } from '@mdi/js'
import type { RouteLocation } from 'vue-router'
import { i18n } from '@/i18n'

export interface NavBarItem {
  icon?: string
  label?: string
  isCurrentUser?: boolean
  isDesktopNoLabel?: boolean
  isTogglelight?: boolean
  isDivider?: boolean
  to?: RouteLocation
  isLogout?: boolean
  isUpload?: boolean
  isCreateDirectory?: boolean
  menu?: NavBarItem[]
  [key: string]: any
}

export default [
  {
    icon: mdiThemeLightDark,
    label: i18n.global.t('nav.menu.theme'),
    isDesktopNoLabel: true,
    isTogglelight: true,
    testid: 'theme-toggle'
  },
  {
    icon: mdiLock,
    label: i18n.global.t('nav.menu.lock'),
    isDesktopNoLabel: true,
    to: { name: 'lock' }
  }
] as NavBarItem[]
