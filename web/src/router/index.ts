import { createRouter, createWebHistory } from 'vue-router'
import IndexView from '../views/files/IndexView.vue'

const router = createRouter({
  history: createWebHistory(`/`),
  routes: [
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
    {
      path: '/links',
      name: 'links',
      meta: {
        files: true,
        title: 'Links'
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
      path: '/auth/login/private-key',
      name: 'login-private-key',
      meta: { title: 'Login' },
      component: () => import('../views/auth/login/PrivateKeyView.vue')
    },
    {
      path: '/auth/register/two-factor',
      name: 'register-two-factor',
      meta: { title: 'Create Account - Two Factor Authentication' },
      component: () => import('../views/auth/register/TwoFactorView.vue')
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
