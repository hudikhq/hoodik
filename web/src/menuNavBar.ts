import { mdiLock } from '@mdi/js'
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
    icon: mdiLock,
    label: i18n.global.t('nav.menu.lock'),
    isDesktopNoLabel: true,
    to: { name: 'lock' }
  }
] as NavBarItem[]
