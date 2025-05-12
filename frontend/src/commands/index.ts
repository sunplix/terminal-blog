import { ref } from 'vue'
import type { Ref } from 'vue'

// 命令处理状态
export interface CommandState {
  showHint: Ref<boolean>
  showCaptcha: Ref<boolean>
  captchaData: Ref<{ captcha: string; sessionId: string } | null>
  commandHint: Ref<string>
  isLoading: Ref<boolean>
  isClearing: Ref<boolean>
  // 新增：记录上次命令名，避免重复获取验证码和提示
  lastCommandName: Ref<string | null>
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
  isLoading: ref(false),
  isClearing: ref(false),
  lastCommandName: ref(null)
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

// 命令描述缓存
export const commandDescriptions: Map<string, string> = new Map()

// 解析help命令返回的消息，提取所有命令描述
export const parseHelpMessage = (message: string) => {
  const descriptions = new Map<string, string>()
  const lines = message.split('\n')
  
  for (const line of lines) {
    if (line.startsWith('- ')) {
      const match = line.match(/^- (\w+): (.+)$/)
      if (match) {
        const [_, command, description] = match
        descriptions.set(command, description)
      }
    }
  }
  
  return descriptions
}

// 初始化命令描述缓存
export const initCommandDescriptions = async () => {
  try {
    const response = await fetch('http://localhost:8080/api/command', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ command: 'help' })
    })
    
    const data = await response.json()
    if (data.success) {
      const descriptions = parseHelpMessage(data.message)
      // 更新缓存
      descriptions.forEach((value, key) => {
        commandDescriptions.set(key, value)
      })
    }
  } catch (error) {
    console.error('获取命令描述失败:', error)
  }
}

// 发送命令到服务器
export const sendCommand = async (
  command: string,
  sessionId: string = '',
  authToken: string | null = null
): Promise<CommandResult> => {
  try {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json'
    }

    if (authToken) {
      headers['Authorization'] = `Bearer ${authToken}`
    }

    const response = await fetch('http://localhost:8080/api/command', {
      method: 'POST',
      headers,
      body: JSON.stringify({
        command,
        session_id: sessionId,
        cwd: currentPath.value
      })
    })

    const data = await response.json()
    return data
  } catch (error) {
    console.error('发送命令失败:', error)
    return {
      success: false,
      message: '网络错误'
    }
  }
}

// 处理命令输入
export const handleCommandInput = async (
  input: string,
  state: CommandState,
  authToken: Ref<string | null>
): Promise<void> => {
  const firstSpaceIdx = input.indexOf(' ')
  if (firstSpaceIdx > -1) {
    const commandName = input.slice(0, firstSpaceIdx).trim()
    if (commandName) {
      // 已登录用户不能使用 login 和 register 命令
      if (authToken.value && (commandName === 'login' || commandName === 'register')) {
        state.showHint.value = true
        state.commandHint.value = '您已登录，请先登出后再尝试登录或注册'
        return
      }

      // 控制验证码显示：首次切换到 login/register 获取
      if ((commandName === 'login' || commandName === 'register') && !authToken.value) {
        if (state.lastCommandName.value !== commandName) {
          state.showCaptcha.value = true
          state.captchaData.value = await getCaptcha()
        }
      } else {
        state.showCaptcha.value = false
        state.captchaData.value = null
      }

      // 控制命令提示：只有命令名变化时才重新获取描述
      if (state.lastCommandName.value !== commandName) {
        state.isLoading.value = true
        state.commandHint.value = ''

        try {
          // 从缓存中获取命令描述
          if (commandDescriptions.has(commandName)) {
            state.showHint.value = true
            state.commandHint.value = commandDescriptions.get(commandName) || ''
          } else {
            // 如果缓存中没有，则重新获取所有命令描述
            // await initCommandDescriptions()
            if (commandDescriptions.has(commandName)) {
              state.showHint.value = true
              state.commandHint.value = commandDescriptions.get(commandName) || ''
            } else {
              state.showHint.value = false
            }
          }
        } finally {
          state.isLoading.value = false
        }
      }

      // 更新上次命令名
      state.lastCommandName.value = commandName
      return
    }
  }

  // 无效输入时重置状态
  resetCommandState(state)
}

// 当前路径状态
const username = localStorage.getItem('username')
export const currentPath = ref(username ? `/home/${username}/` : '/home/guest/')

// 更新路径
export const updatePath = (newPath: string) => {
  currentPath.value = newPath
  localStorage.setItem('cwd', newPath)
}

// 获取文件类型emoji
const getFileEmoji = (name: string, isDirectory: boolean): string => {
  if (isDirectory) return '📂'
  
  const ext = name.split('.').pop()?.toLowerCase()
  switch (ext) {
    case 'txt': return '📄'
    case 'md': return '📝'
    case 'json': return '📋'
    case 'js': case 'ts': return '📜'
    case 'css': case 'scss': return '🎨'
    case 'html': case 'vue': return '🌐'
    case 'jpg': case 'jpeg': case 'png': case 'gif': return '🖼️'
    case 'mp3': case 'wav': return '🎵'
    case 'mp4': case 'avi': return '🎬'
    case 'zip': case 'rar': case '7z': return '📦'
    case 'pdf': return '📑'
    default: return '📎'
  }
}

// 格式化ls命令输出
const formatLsOutput = (contents: any[], path: string): string => {
  const cwd = localStorage.getItem('cwd') || '/home/guest/'
  const isRoot = cwd === '/home/guest/' || cwd.endsWith('/')
  
  return contents.map(item => {
    const name = item.name.split('/').pop() || item.name
    const emoji = getFileEmoji(name, item.is_directory)
    const permissions = item.permissions
    const date = new Date(item.updated_at).toLocaleString()
    const size = item.is_directory ? '<DIR>' : '0'
    return `${emoji} ${permissions} ${date} ${size.padStart(8)} ${name}`
  }).join('\n')
}

// 执行命令
export const executeCommand = async (
  command: string,
  state: CommandState,
  authToken: Ref<string | null>,
  onOutput: (text: string, isError: boolean, isCaptcha: boolean) => void
): Promise<void> => {
  if (state.isClearing.value) return

  // 检查已登录用户不能使用 login 和 register 命令
  const commandName = command.split(' ')[0]
  const username = command.split(' ')[1]
  if (authToken.value && (commandName === 'login' || commandName === 'register')) {
    onOutput('您已登录，请先登出后再尝试登录或注册', true, false)
    return
  }

  const sessionId = state.captchaData.value?.sessionId || ''
  const response = await sendCommand(command, sessionId, authToken.value)

  // 特殊处理ls命令的输出
  if (commandName === 'ls' && response.success && response.data?.contents) {
    const formattedOutput = formatLsOutput(response.data.contents, response.data.path)
    onOutput(formattedOutput, false, false)
  } else {
    onOutput(response.message, !response.success, false)
  }
  if (command.startsWith('cd') && response.success && response.data?.token) {
    updatePath(response.data.path)
  }
  if (command.startsWith('login ') && response.success && response.data?.token) {
    const token = response.data.token
    localStorage.setItem('token', token)
    localStorage.setItem('username', username)
    authToken.value = token
    // 更新路径
    updatePath(`/home/${username}/`)
  } else if (command === 'logout' && response.success) {
    localStorage.removeItem('token')
    localStorage.removeItem('username')
    authToken.value = null
    // 更新路径
    updatePath('/home/guest/')
  } else if (command.startsWith('cd ') && response.success) {
    // 更新路径
    updatePath(response.data.path)
  }
}

// 重置命令状态
export const resetCommandState = (state: CommandState): void => {
  state.showHint.value = false
  state.showCaptcha.value = false
  state.commandHint.value = ''
  state.isLoading.value = false
  state.captchaData.value = null
  state.lastCommandName.value = null
}

// 关闭提示框
export const closeHint = (state: CommandState): void => {
  resetCommandState(state)
}

// 清除输出
export const clearOutput = (state: CommandState): void => {
  state.isClearing.value = true
}
