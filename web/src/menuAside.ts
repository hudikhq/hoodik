import {
  mdiMonitor,
  mdiShareVariantOutline,
  mdiHuman,
  mdiHumanMaleFemale,
  mdiCog,
  mdiFileDocumentOutline
} from '@mdi/js'
import type { RouteLocation } from 'vue-router'
import { i18n } from '@/i18n'

export interface AsideMenuItemType {
  to: RouteLocation
  icon: string
  label: string
  roles?: string[]
  expandable?: boolean
}

export default [
  {
    to: { name: 'files' },
    icon: mdiMonitor,
    label: i18n.global.t('nav.menu.files'),
    expandable: true
  },
  {
    to: { name: 'notes' },
    icon: mdiFileDocumentOutline,
    label: i18n.global.t('nav.menu.notes')
  },
  {
    to: { name: 'share' },
    icon: mdiShareVariantOutline,
    label: i18n.global.t('nav.menu.share')
  },
  {
    to: { name: 'account' },
    icon: mdiHuman,
    label: i18n.global.t('nav.menu.account')
  },
  {
    to: { name: 'manage-users' },
    icon: mdiHumanMaleFemale,
    label: i18n.global.t('nav.menu.users'),
    roles: ['admin']
  },
  {
    to: { name: 'manage-settings' },
    icon: mdiCog,
    label: i18n.global.t('common.settings'),
    roles: ['admin']
  }
] as AsideMenuItemType[]
