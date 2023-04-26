<script setup lang="ts">
import { computed } from 'vue'
import { mdiCashMinus, mdiCashPlus, mdiReceipt, mdiCreditCardOutline } from '@mdi/js'
import CardBox from '@/components/ui/CardBox.vue'
import BaseLevel from '@/components/ui/BaseLevel.vue'
import PillTag from '@/components/ui/PillTag.vue'
import IconRounded from '@/components/ui/IconRounded.vue'
import type { ColorType } from '@/colors'

const props = defineProps({
  amount: {
    type: Number,
    required: true
  },
  date: {
    type: String,
    required: true
  },
  business: {
    type: String,
    required: true
  },
  type: {
    type: String,
    required: true
  },
  name: {
    type: String,
    required: true
  },
  account: {
    type: String,
    required: true
  }
})

const icon = computed((): { icon: any; type: ColorType } => {
  if (props.type === 'withdrawal') {
    return {
      icon: mdiCashMinus,
      type: 'danger'
    }
  } else if (props.type === 'deposit') {
    return {
      icon: mdiCashPlus,
      type: 'success'
    }
  } else if (props.type === 'invoice') {
    return {
      icon: mdiReceipt,
      type: 'warning'
    }
  }

  return {
    icon: mdiCreditCardOutline,
    type: 'info'
  }
})
</script>

<template>
  <CardBox class="mb-6 last:mb-0" is-hoverable>
    <BaseLevel>
      <BaseLevel type="justify-start">
        <IconRounded :icon="icon.icon" :color="icon.type" class="md:mr-6" />
        <div class="text-center space-y-1 md:text-left md:mr-6">
          <h4 class="text-xl">${{ amount }}</h4>
          <p class="text-brownish-500 dark:text-brownish-400">
            <b>{{ date }}</b> via {{ business }}
          </p>
        </div>
      </BaseLevel>
      <div class="text-center md:text-right space-y-2">
        <p class="text-sm text-brownish-500">
          {{ name }}
        </p>
        <div>
          <PillTag :color="icon.type" :label="type" small />
        </div>
      </div>
    </BaseLevel>
  </CardBox>
</template>
