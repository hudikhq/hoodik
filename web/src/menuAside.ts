import {
  mdiMonitor,
  mdiShareVariantOutline,
  mdiHuman,
  mdiHumanMaleFemale,
  mdiCog,
  mdiFileDocumentOutline
} from '@mdi/js'
import type { RouteLocation } from 'vue-router'

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
    label: 'Files',
    expandable: true
  },
  {
    to: { name: 'notes' },
    icon: mdiFileDocumentOutline,
    label: 'Notes'
  },
  {
    to: { name: 'share' },
    icon: mdiShareVariantOutline,
    label: 'Share'
  },
  {
    to: { name: 'account' },
    icon: mdiHuman,
    label: 'Account'
  },
  {
    to: { name: 'manage-users' },
    icon: mdiHumanMaleFemale,
    label: 'Users',
    roles: ['admin']
  },
  {
    to: { name: 'manage-settings' },
    icon: mdiCog,
    label: 'Settings',
    roles: ['admin']
  }
] as AsideMenuItemType[]
