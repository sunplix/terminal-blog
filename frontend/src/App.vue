<script setup lang="ts">
import { ref, onMounted, type Ref } from 'vue'
import './styles/terminal.css'
import * as HintContainer from './components/HintContainer.vue'
import * as CaptchaLabel from './components/CaptchaLabel.vue'
import * as CommandHint from './components/CommandHint.vue'
import * as PathDisplay from './components/PathDisplay.vue'
import { createCommandState, handleCommandInput, closeHint, executeCommand, clearOutput, initCommandDescriptions, type CommandState, currentPath } from './commands'

// 主题状态
const isDarkTheme = ref(true)

// 认证状态
const authToken = ref<string | null>(localStorage.getItem('token'))

// 命令处理状态
const commandState = createCommandState()

// 输出区域引用
const outputArea = ref<HTMLDivElement | null>(null)
const terminalContent = ref<HTMLDivElement | null>(null)
const terminal = ref<HTMLDivElement | null>(null)

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
onMounted(async () => {
  // 初始化命令描述缓存
  await initCommandDescriptions()
  
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

// 创建输出区域
const createOutputArea = () => {
  if (!outputArea.value && terminalContent.value) {
    const area = document.createElement('div')
    area.className = 'output-area'
    area.id = 'terminal-output'
    terminalContent.value.appendChild(area)
    // 触发重排以启动动画
    area.offsetHeight
    area.classList.add('visible')
    outputArea.value = area
  }
  return outputArea.value
}

// 添加输出
const addOutput = (text: string, isError = false, isCaptcha = false) => {
  if (commandState.isClearing.value) return

  const area = createOutputArea()
  if (!area) return

  const output = document.createElement('div')
  output.className = `command-output ${isError ? 'error' : 'success'} ${isCaptcha ? 'captcha' : ''}`
  output.textContent = text
  area.insertBefore(output, area.firstChild)
  area.scrollTop = 0

  // 边框高亮动画
  const inputArea = document.querySelector('.input-area')
  if (inputArea) {
    inputArea.classList.remove('success', 'error')
    // 触发重绘以重置动画
    void (inputArea as HTMLElement).offsetWidth
    if (isError) {
      inputArea.classList.add('error')
    } else if (!isCaptcha) {
      inputArea.classList.add('success')
    }
  }

  adjustTerminalHeight()
}

// 调整终端高度
const adjustTerminalHeight = () => {
  if (outputArea.value && terminal.value) {
    const outputHeight = outputArea.value.scrollHeight
    const maxHeight = window.innerHeight * 0.7 // 70vh
    const newHeight = Math.min(outputHeight + 50, maxHeight) // 50px 是输入框的高度
    terminal.value.style.height = `${newHeight}px`
  }
}

// 重置终端高度
const resetTerminalHeight = () => {
  if (terminal.value) {
    terminal.value.style.height = 'auto'
  }
}

// 清除输出
const handleClearOutput = () => {
  if (!outputArea.value) return

  commandState.isClearing.value = true
  outputArea.value.classList.remove('visible')

  // 等待过渡动画完成后再移除元素
  outputArea.value.addEventListener('transitionend', () => {
    if (outputArea.value && !outputArea.value.classList.contains('visible')) {
      outputArea.value.remove()
      outputArea.value = null
      // 重置终端高度
      resetTerminalHeight()
      commandState.isClearing.value = false
    }
  }, { once: true })
}

const handleInput = async (event: Event) => {
  const input = (event.target as HTMLInputElement).value
  await handleCommandInput(input, commandState, authToken)
}

const handleKeyPress = async (event: KeyboardEvent) => {
  if (event.key === 'Enter') {
    const input = (event.target as HTMLInputElement).value;
    if (input.trim()) {
      if (input === 'clear') {
        handleClearOutput();
        // 手动添加绿光效果
        const inputArea = document.querySelector('.input-area');
        if (inputArea) {
          inputArea.classList.remove('success', 'error');
          // 触发重绘以重置动画
          void (inputArea as HTMLElement).offsetWidth;
          inputArea.classList.add('success');
        }
      } else {
        await executeCommand(input, commandState, authToken, addOutput);
      }
      // 清空输入框
      (event.target as HTMLInputElement).value = '';
    }
  }
};
</script>

<template>
  <div class="container">
    <div class="terminal" ref="terminal">
      <div class="terminal-content" ref="terminalContent">
        <PathDisplay.default :current-path="currentPath" />
        <div class="input-area">
          <span class="prompt">$</span>
          <div class="input-wrapper">
            <input 
              type="text" 
              id="terminal-input" 
              autocomplete="off" 
              spellcheck="false"
              @input="handleInput"
              @keypress="handleKeyPress"
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

