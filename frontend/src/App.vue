<script setup lang="ts">
import { ref, onMounted } from 'vue'
import './styles/terminal.css'
import * as HintContainer from './components/HintContainer.vue'
import * as CaptchaLabel from './components/CaptchaLabel.vue'
import * as CommandHint from './components/CommandHint.vue'
import { createCommandState, handleCommandInput, closeHint } from './commands'

// 主题状态
const isDarkTheme = ref(true)

// 认证状态
const authToken = ref<string | null>(localStorage.getItem('token'))

// 命令处理状态
const commandState = createCommandState()

// 切换主题
const toggleTheme = () => {
  isDarkTheme.value = !isDarkTheme.value
  document.body.setAttribute('data-theme', isDarkTheme.value ? 'dark' : 'light')
  localStorage.setItem('theme', isDarkTheme.value ? 'dark' : 'light')
  
  // 更新背景渐变色
  if (isDarkTheme.value) {
    document.body.style.background = 'linear-gradient(-45deg, #000000, #1a1a1a, #333333, #1a1a1a)'
  } else {
    document.body.style.background = 'linear-gradient(-45deg, #ffffff, #e0e0e0, #c0c0c0, #e0e0e0)'
  }
}

// 初始化主题
onMounted(() => {
  const savedTheme = localStorage.getItem('theme')
  if (savedTheme) {
    isDarkTheme.value = savedTheme === 'dark'
    document.body.setAttribute('data-theme', savedTheme)
    
    // 初始化背景渐变色
    if (isDarkTheme.value) {
      document.body.style.background = 'linear-gradient(-45deg, #000000, #1a1a1a, #333333, #1a1a1a)'
    } else {
      document.body.style.background = 'linear-gradient(-45deg, #ffffff, #e0e0e0, #c0c0c0, #e0e0e0)'
    }
  }
})

const handleInput = async (event: Event) => {
  const input = (event.target as HTMLInputElement).value
  await handleCommandInput(input, commandState, authToken.value)
}
</script>

<template>
  <div class="container">
    <div class="terminal">
      <div class="terminal-content">
        <div class="input-area">
          <span class="prompt">$</span>
          <div class="input-wrapper">
            <input 
              type="text" 
              id="terminal-input" 
              autocomplete="off" 
              spellcheck="false"
              @input="handleInput"
            >
          </div>
          <button id="theme-toggle" class="theme-toggle" @click="toggleTheme">
            <i :class="isDarkTheme ? 'ri-sun-line' : 'ri-moon-line'"></i>
          </button>
        </div>
        
        <HintContainer.default :visible="commandState.showHint.value" @close="() => closeHint(commandState)">
          <CaptchaLabel.default 
            v-if="commandState.showCaptcha.value && commandState.captchaData.value" 
            :captcha="commandState.captchaData.value.captcha"
            :session-id="commandState.captchaData.value.sessionId"
          />
          <CommandHint.default 
            :hint="commandState.commandHint.value"
            :visible="!!commandState.commandHint.value || commandState.isLoading.value"
            :loading="commandState.isLoading.value"
          />
        </HintContainer.default>
      </div>
    </div>
  </div>
</template>

