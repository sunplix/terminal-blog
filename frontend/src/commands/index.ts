import { ref } from 'vue'
import type { Ref } from 'vue'

// 命令处理状态
export interface CommandState {
  showHint: Ref<boolean>
  showCaptcha: Ref<boolean>
  captchaData: Ref<{ captcha: string; sessionId: string } | null>
  commandHint: Ref<string>
  isLoading: Ref<boolean>
}

// 命令处理结果
export interface CommandResult {
  success: boolean
  message: string
  data?: any
}

// 创建命令处理状态
export const createCommandState = (): CommandState => ({
  showHint: ref(false),
  showCaptcha: ref(false),
  captchaData: ref<{ captcha: string; sessionId: string } | null>(null),
  commandHint: ref(''),
  isLoading: ref(false)
})

// 获取验证码
export const getCaptcha = async (): Promise<{ captcha: string; sessionId: string } | null> => {
  try {
    const response = await fetch('http://localhost:8080/api/captcha')
    const data = await response.json()
    if (data.success) {
      return {
        captcha: data.data.captcha,
        sessionId: data.data.session_id
      }
    }
    return null
  } catch (error) {
    console.error('获取验证码失败:', error)
    return null
  }
}

// 获取命令描述
export const getCommandDescription = async (commandName: string): Promise<string> => {
  try {
    const response = await fetch('http://localhost:8080/api/command', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ command: `description ${commandName}` })
    })

    const data = await response.json()
    return data.success ? data.message : `未知命令: ${commandName}`
  } catch (error) {
    console.error('获取命令描述失败:', error)
    return '获取命令描述失败'
  }
}

// 处理命令输入
export const handleCommandInput = async (
  input: string,
  state: CommandState,
  authToken: string | null
): Promise<void> => {
  // 找到第一个空格的位置
  const firstSpaceIdx = input.indexOf(' ')
  if (firstSpaceIdx > -1) {
    // 取出空格前的命令名
    const commandName = input.slice(0, firstSpaceIdx).trim()
    if (commandName) {
      // 对于 login 和 register 命令，保持原有的验证码显示逻辑
      if ((commandName === 'login' || commandName === 'register') && !authToken) {
        state.showCaptcha.value = true
        // 每次检测到命令时都重新获取验证码
        state.captchaData.value = await getCaptcha()
      } else {
        state.showCaptcha.value = false
        state.captchaData.value = null
      }

      // 显示命令提示
      state.showHint.value = true
      state.isLoading.value = true
      state.commandHint.value = ''

      try {
        state.commandHint.value = await getCommandDescription(commandName)
      } finally {
        state.isLoading.value = false
      }
      return
    }
  }

  // 没有空格或空格前是空字符串，都不显示提示
  resetCommandState(state)
}

// 重置命令状态
export const resetCommandState = (state: CommandState): void => {
  state.showHint.value = false
  state.showCaptcha.value = false
  state.commandHint.value = ''
  state.isLoading.value = false
  state.captchaData.value = null
}

// 关闭提示框
export const closeHint = (state: CommandState): void => {
  resetCommandState(state)
} 