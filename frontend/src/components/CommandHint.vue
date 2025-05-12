<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'

const props = defineProps<{
  hint: string
  visible: boolean
  loading?: boolean
}>()

const hintEl = ref<HTMLElement | null>(null)

onMounted(() => {
  if (hintEl.value) {
    hintEl.value.style.opacity = '0'
    hintEl.value.style.transform = 'translateY(-10px)'
    setTimeout(() => {
      if (hintEl.value) {
        hintEl.value.style.opacity = '1'
        hintEl.value.style.transform = 'translateY(0)'
      }
    }, 10)
  }
})

watch(() => props.visible, (newValue) => {
  if (hintEl.value) {
    if (newValue) {
      // 显示时的动画
      hintEl.value.style.display = 'block'
      setTimeout(() => {
        if (hintEl.value) {
          hintEl.value.style.opacity = '1'
          hintEl.value.style.transform = 'translateY(0)'
        }
      }, 10)
    } else {
      // 隐藏时的动画
      hintEl.value.style.opacity = '0'
      hintEl.value.style.transform = 'translateY(-10px)'
      setTimeout(() => {
        if (hintEl.value) {
          hintEl.value.style.display = 'none'
        }
      }, 300) // 等待淡出动画完成
    }
  }
})
</script>

<template>
  <div ref="hintEl" class="command-hint" :class="{ 'visible': props.visible }">
    <div v-if="props.loading" class="loading-dots">
      <span>.</span><span>.</span><span>.</span>
    </div>
    <div v-else>{{ props.hint }}</div>
  </div>
</template>

<style scoped>
.command-hint {
  color: var(--text-color);
  font-family: 'JetBrains Mono', "PingFang SC", "Microsoft YaHei", "Helvetica Neue", Helvetica, Arial, sans-serif;
  font-size: 14px;
  padding: 4px 0;
  position: relative;
  background: none;
  background-image: linear-gradient(to right, var(--text-color), var(--accent-color), var(--text-color));
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  white-space: pre-wrap;
  word-break: break-word;
  opacity: 0;
  transform: translateY(-10px);
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  display: none;
}

.command-hint.visible {
  display: block;
  opacity: 1;
  transform: translateY(0);
}

.loading-dots {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 20px;
}

.loading-dots span {
  animation: loading 1.4s infinite;
  opacity: 0;
  margin: 0 2px;
}

.loading-dots span:nth-child(2) {
  animation-delay: 0.2s;
}

.loading-dots span:nth-child(3) {
  animation-delay: 0.4s;
}

@keyframes loading {
  0% { opacity: 0; }
  50% { opacity: 1; }
  100% { opacity: 0; }
}

@keyframes shine {
  0% { background-position: 100% 0; }
  50% { background-position: 0% 0; }
  100% { background-position: -100% 0; }
}

.command-hint.visible {
  animation: shine 3.6s ease-out infinite;
}
</style> 