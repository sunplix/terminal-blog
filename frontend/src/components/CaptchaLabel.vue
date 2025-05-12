<script setup lang="ts">
import { ref, onMounted } from 'vue'

const props = defineProps<{
  captcha: string
  sessionId: string
}>()

const label = ref<HTMLElement | null>(null)

onMounted(() => {
  if (label.value) {
    label.value.style.opacity = '0'
    label.value.style.transform = 'translateY(-10px)'
    setTimeout(() => {
      if (label.value) {
        label.value.style.opacity = '1'
        label.value.style.transform = 'translateY(0)'
      }
    }, 10)
  }
})
</script>

<template>
  <div ref="label" class="captcha-label" :data-session-id="sessionId">
    验证码: {{ captcha }}
  </div>
</template>

<style scoped>
.captcha-label {
  color: var(--error-color);
  font-weight: bold;
  font-family: 'JetBrains Mono', "PingFang SC", "Microsoft YaHei", "Helvetica Neue", Helvetica, Arial, sans-serif;
  letter-spacing: 2px;
  padding: 4px 0;
  margin-bottom: 4px;
  transition: opacity 0.3s, transform 0.3s;
  user-select: all;
  cursor: pointer;
}
</style> 