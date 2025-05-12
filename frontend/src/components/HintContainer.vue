<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'

const props = defineProps<{
  visible: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const container = ref<HTMLElement | null>(null)

onMounted(() => {
  if (container.value) {
    container.value.style.opacity = '0'
    container.value.style.transform = 'translateY(-10px)'
    setTimeout(() => {
      if (container.value) {
        container.value.style.opacity = '1'
        container.value.style.transform = 'translateY(0)'
      }
    }, 10)
  }
})

const handleClose = () => {
  if (container.value) {
    container.value.style.opacity = '0'
    container.value.style.transform = 'translateY(-10px)'
    setTimeout(() => {
      emit('close')
    }, 300)
  }
}
</script>

<template>
  <div v-if="visible" ref="container" class="hint-container">
    <slot></slot>
  </div>
</template>

<style scoped>
.hint-container {
  padding: 8px 20px;
  background-color: var(--input-bg);
  backdrop-filter: blur(10px);
  -webkit-backdrop-filter: blur(10px);
  border-radius: 12px;
  margin: 8px 0;
  transition: all 0.3s ease;
  border: none;
}
</style> 