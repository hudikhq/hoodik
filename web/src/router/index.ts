import { createRouter, createWebHistory } from 'vue-router'
import FilesView from '../views/FilesView.vue'

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
      component: FilesView
    },
    {
      path: '/auth/setup-lock-screen',
      name: 'setup-lock-screen',
      meta: { title: 'Setup Lock Screen' },
      component: () => import('../views/auth/SetupLockScreenView.vue')
    },
    {
      path: '/auth/decrypt',
      name: 'decrypt',
      meta: { title: 'Decrypt Private Key' },
      component: () => import('../views/auth/DecryptView.vue')
    },
    {
      path: '/auth/lock',
      name: 'lock',
      meta: { title: 'Account Locked' },
      component: () => import('../views/auth/LockView.vue')
    },
    {
      path: '/auth/login/private-key',
      name: 'login-private-key',
      meta: { title: 'Login' },
      component: () => import('../views/auth/login/PrivateKeyView.vue')
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
