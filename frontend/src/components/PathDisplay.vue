<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'

const props = defineProps<{
  currentPath: string
}>()

const pathDisplay = ref<HTMLElement | null>(null)

onMounted(() => {
  if (pathDisplay.value) {
    pathDisplay.value.style.opacity = '0'
    pathDisplay.value.style.transform = 'translateY(-10px)'
    setTimeout(() => {
      if (pathDisplay.value) {
        pathDisplay.value.style.opacity = '1'
        pathDisplay.value.style.transform = 'translateY(0)'
      }
    }, 10)
  }
})

// 监听路径变化
watch(() => props.currentPath, (newPath) => {
  if (pathDisplay.value) {
    // 添加过渡动画
    pathDisplay.value.style.opacity = '0'
    pathDisplay.value.style.transform = 'translateY(-10px)'
    
    setTimeout(() => {
      if (pathDisplay.value) {
        pathDisplay.value.style.opacity = '1'
        pathDisplay.value.style.transform = 'translateY(0)'
      }
    }, 10)
  }
})
</script>

<template>
  <div ref="pathDisplay" class="path-display">
    {{ props.currentPath }}
  </div>
</template>

<style scoped>
.path-display {
  color: var(--text-color);
  font-family: 'JetBrains Mono', "PingFang SC", "Microsoft YaHei", "Helvetica Neue", Helvetica, Arial, sans-serif;
  font-size: 14px;
  padding: 4px 0;
  opacity: 0;
  margin-left: 15px;
  transform: translateY(-10px);
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  position: relative;
  background: none;
  background-image: linear-gradient(
    to right,
    #EEC9A3 0%,
    #EF629F 50%,
    #EEC9A3 100%
  );
  background-size: 200% auto;
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  white-space: pre-wrap;
  word-break: break-word;
  animation: textShine 3s linear infinite;
}

@keyframes textShine {
  to {
    background-position: 200% center;
  }
}
</style> 