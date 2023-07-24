import { createRouter, createWebHistory } from 'vue-router'
import IndexView from '../views/files/IndexView.vue'

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
      component: IndexView
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

    /**
     * Link routes
     */
    {
      path: '/links',
      name: 'links',
      meta: {
        files: true,
        title: 'My Links'
      },
      component: () => import('../views/links/IndexView.vue')
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

export default router
