import { createRouter, createWebHistory } from 'vue-router'

import { capabilitiesStore } from '!/shares'
import { SHARED_WITH_ME_DIR_ID } from '!/storage'

const router = createRouter({
  history: createWebHistory(`/`),
  routes: [
    /**
     * File routes
     */
    {
      path: '/:file_id?',
      name: 'files',
      meta: {
        files: true,
        title: 'My files'
      },
      component: () => import('../views/files/IndexView.vue')
    },
    {
      path: '/p/:id',
      name: 'file-preview',
      meta: {
        files: true,
        title: 'File Preview'
      },
      component: () => import('../views/files/FileView.vue')
    },
    {
      path: '/notes/:id?',
      name: 'notes',
      meta: {
        files: true,
        title: 'Notes'
      },
      component: () => import('../views/notes/NotesView.vue')
    },

    /**
     * Share hub routes — parent layout renders the sub-tab control and a
     * nested `<RouterView />`. After the virtual "Shared with me" folder
     * and the unified per-file Sharing modal landed, the hub trims to
     * three surfaces: Public links, Activity (audit log), Groups.
     *
     * The bare paths for the retired sub-tabs (`/share/with-me`,
     * `/share/mine`) and the old audit path (`/share/audit`) stay
     * registered as redirects so bookmarks survive.
     */
    {
      path: '/share',
      meta: { title: 'Share', files: true },
      component: () => import('../views/shares/ShareHub.vue'),
      children: [
        {
          path: '',
          name: 'share',
          redirect: { name: 'share-public' }
        },
        {
          path: 'public',
          name: 'share-public',
          component: () => import('../views/shares/ShareHubPublic.vue'),
          meta: { title: 'Public links' }
        },
        {
          path: 'with-me',
          redirect: { name: 'files', params: { file_id: SHARED_WITH_ME_DIR_ID } }
        },
        {
          path: 'mine',
          redirect: { name: 'share-public' }
        },
        {
          path: 'audit',
          redirect: { name: 'share-activity' }
        },
        {
          path: 'activity',
          name: 'share-activity',
          component: () => import('../views/shares/ShareHubAudit.vue'),
          meta: { title: 'Activity', requiresSharing: true, requiresAuditLog: true }
        },
        {
          path: 'groups',
          name: 'share-groups',
          component: () => import('../views/shares/ShareHubGroups.vue'),
          meta: { title: 'Groups', requiresSharing: true, requiresGroups: true }
        }
      ]
    },

    /**
     * Backwards-compatibility redirect from the retired /links tree.
     * Every `/links*` URL maps to `/share/public*`.
     */
    {
      path: '/links/:pathMatch(.*)*',
      name: 'links-legacy',
      redirect: (to) => {
        const tail = Array.isArray(to.params.pathMatch)
          ? to.params.pathMatch.join('/')
          : (to.params.pathMatch ?? '')
        const suffix = tail ? `/${tail}` : ''
        return { path: `/share/public${suffix}`, query: to.query, hash: to.hash }
      }
    },
    {
      path: '/l/:link_id',
      name: 'links-view',
      meta: {
        files: true,
        title: 'File Link'
      },
      component: () => import('../views/links/LinkView.vue')
    },

    /**
     * Account routes
     */
    {
      path: '/account',
      name: 'account',
      meta: { title: 'My Account' },
      component: () => import('../views/account/IndexView.vue')
    },
    {
      path: '/account/change-password',
      name: 'account-change-password',
      meta: { title: 'Change my password' },
      component: () => import('../views/account/ChangePasswordView.vue')
    },

    /**
     * Admin routes
     */
    {
      path: '/manage/users',
      name: 'manage-users',
      meta: { title: 'Manage Users' },
      component: () => import('../views/admin/IndexView.vue')
    },
    {
      path: '/manage/users/:id',
      name: 'manage-users-single',
      meta: { title: 'Manage User' },
      component: () => import('../views/admin/UsersSingleView.vue')
    },
    {
      path: '/manage/settings',
      name: 'manage-settings',
      meta: { title: 'Manage Settings' },
      component: () => import('../views/admin/SettingsView.vue')
    },

    /**
     * Auth routes
     */
    {
      path: '/auth/pin/lock',
      name: 'lock',
      meta: { title: 'Account Locked' },
      component: () => import('../views/auth/pin/IndexView.vue')
    },
    {
      path: '/auth/pin/setup-lock-screen',
      name: 'setup-lock-screen',
      meta: { title: 'Setup Lock Screen' },
      component: () => import('../views/auth/pin/SetupLockScreenView.vue')
    },
    {
      path: '/auth/decrypt',
      name: 'decrypt',
      meta: { title: 'Decrypt Private Key' },
      component: () => import('../views/auth/pin/DecryptView.vue')
    },
    {
      path: '/auth/login',
      name: 'login',
      meta: { title: 'Login - Credentials' },
      component: () => import('../views/auth/login/IndexView.vue')
    },
    {
      path: '/auth/login/private-key',
      name: 'login-private-key',
      meta: { title: 'Login' },
      component: () => import('../views/auth/login/PrivateKeyView.vue')
    },
    {
      path: '/auth/forgot-password',
      name: 'forgot-password',
      meta: { title: 'Recover your password' },
      component: () => import('../views/auth/ForgotPasswordView.vue')
    },
    {
      path: '/auth/register',
      name: 'register',
      meta: { title: 'Create Account - Credentials' },
      component: () => import('../views/auth/register/IndexView.vue')
    },
    {
      path: '/auth/register/key',
      name: 'register-key',
      meta: { title: 'Create Account - Your Private Key' },
      component: () => import('../views/auth/register/KeyView.vue')
    },
    {
      path: '/auth/register/two-factor',
      name: 'register-two-factor',
      meta: { title: 'Create Account - Two Factor Authentication' },
      component: () => import('../views/auth/register/TwoFactorView.vue')
    },
    {
      path: '/auth/register/resend',
      name: 'register-resend-activation',
      meta: { title: 'Create Account - Resend Activation Email' },
      component: () => import('../views/auth/register/ResendActivation.vue')
    },
    {
      path: '/auth/activate-email/:token',
      name: 'activate-email',
      meta: { title: 'Create Account - Verify Email' },
      component: () => import('../views/auth/VerifyEmailView.vue')
    }
  ]
})

/**
 * Sub-tabs of the Share hub that require account-to-account sharing
 * to be enabled by the server (capability fetch). Public links are
 * always available — the Public tab stays visible even when the kill
 * switch is on.
 *
 * If the capability fetch hasn't landed yet (cold navigation), wait for
 * the first response rather than redirecting on a stale `false`. The
 * fail-closed defaults stay in effect on fetch error.
 */
router.beforeEach(async (to) => {
  if (to.meta?.requiresSharing !== true) return true
  const caps = capabilitiesStore()
  if (caps.lastFetchedAt === null) {
    try {
      await caps.fetch()
    } catch {
      // Fail-closed defaults already installed inside the store.
    }
  }
  if (!caps.sharingEnabled) return { name: 'share-public' }
  if (to.meta?.requiresAuditLog === true && !caps.auditLog) return { name: 'share-public' }
  if (to.meta?.requiresGroups === true && !caps.shareGroups) return { name: 'share-public' }
  return true
})

export default router
