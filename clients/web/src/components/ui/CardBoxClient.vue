<script setup lang="ts">
import { computed } from 'vue'
import { mdiTrendingDown, mdiTrendingUp, mdiTrendingNeutral } from '@mdi/js'
import CardBox from '@/components/ui/CardBox.vue'
import BaseLevel from '@/components/ui/BaseLevel.vue'
import PillTag from '@/components/ui/PillTag.vue'
import UserAvatar from '@/components/ui/UserAvatar.vue'
import type { ColorType } from '@/colors'

const props = defineProps<{
  name: string
  login: string
  date: string
  progress?: number
  text?: string
  type?: ColorType
}>()

const pillType = computed((): ColorType => {
  if (props.type) {
    return props.type
  }

  if (props.progress) {
    if (props.progress >= 60) {
      return 'success'
    }
    if (props.progress >= 40) {
      return 'warning'
    }

    return 'danger'
  }

  return 'info'
})

const pillIcon = computed(() => {
  return (
    {
      success: mdiTrendingUp,
      warning: mdiTrendingNeutral,
      danger: mdiTrendingDown,
      info: null,
      whiteDark: null,
      white: null,
      lightDark: null,
      contrast: null,
      light: null
    }[pillType.value] || undefined
  )
})

const pillText = computed(() => props.text ?? `${props.progress}%`)
</script>

<template>
  <CardBox class="mb-6 last:mb-0" is-hoverable>
    <BaseLevel>
      <BaseLevel type="justify-start">
        <UserAvatar class="w-12 h-12 mr-6" :username="name" />
        <div class="text-center md:text-left overflow-hidden">
          <h4 class="text-xl text-ellipsis">
            {{ name }}
          </h4>
          <p class="text-brownish-500 dark:text-brownish-400">{{ date }} @ {{ login }}</p>
        </div>
      </BaseLevel>
      <PillTag :color="pillType" :label="pillText" :icon="pillIcon" />
    </BaseLevel>
  </CardBox>
</template>
