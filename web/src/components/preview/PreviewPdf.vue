<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { Preview } from '!/preview'
import PdfApp from 'vue3-pdf-app'
import 'vue3-pdf-app/dist/icons/main.css'

const props = defineProps<{
  modelValue: Preview
}>()
const preview = computed(() => props.modelValue)

const data = ref()

const config = ref({
  toolbar: {
    toolbarViewerRight: false
  },
  secondaryToolbar: {
    documentProperties: false
  }
})

const load = async () => {
  data.value = (await preview.value.load()).buffer
}

watch(
  () => props.modelValue,
  () => setTimeout(load, 100),
  { immediate: true }
)
</script>

<template>
  <PdfApp
    style="width: 100%"
    :pdf="data"
    :file-name="preview.name"
    :config="config"
    theme="dark"
  ></PdfApp>
</template>
<style scoped>
.pdf-app.dark {
  --pdf-app-background-color: #0a0908;
  --pdf-toolbar-color: #232323;
}
.pdf-app.light {
  --pdf-app-background-color: #0a0908;
  --pdf-toolbar-color: #232323;
}
</style>
