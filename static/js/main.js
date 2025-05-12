document.addEventListener('DOMContentLoaded', async () => {
    const terminalInput = document.getElementById('terminal-input');
    const terminalContent = document.querySelector('.terminal-content');
    const terminal = document.querySelector('.terminal');
    const themeToggle = document.getElementById('theme-toggle');
    const themeIcon = themeToggle.querySelector('i');
    const inputArea = document.querySelector('.input-area');
    let commandHistory = [];
    let historyIndex = -1;
    let outputArea = null;
    let isClearing = false;
    let captcha = null;
    let captchaLabel = null;
    let commandHint = null;
    let hintContainer = null;

    // 存储 token
    let authToken = localStorage.getItem('token');

    // 虚拟文件系统相关
    let currentPath = '/';
    let username = null;
    let pathDisplay = null;

    // 初始化虚拟文件系统
    await checkAndCreateVirtualDirectories();
    updatePathDisplay();

    // 虚拟文件系统结构
    const virtualFileSystem = {
        // 获取目录内容
        getDirectoryContents(path) {
            // 访客模式
            if (!username) {
                if (path === '/home/guest/') {
                    return {
                        type: 'directory',
                        contents: ['Desktop/', 'Documents/', 'Album/', 'Config/', 'Trash/']
                    };
                }
                return { type: 'directory', contents: [] };
            }

            // 用户模式
            if (path === `/home/${username}/`) {
                return {
                    type: 'directory',
                    contents: ['Desktop/', 'Documents/', 'Album/', 'Config/', 'Trash/']
                };
            } else if (path === `/home/${username}/Documents/`) {
                return {
                    type: 'directory',
                    contents: ['drafts/', 'published/']
                };
            } else if (path === `/home/${username}/Album/`) {
                return {
                    type: 'directory',
                    contents: ['avatars/', 'covers/', 'uploads/']
                };
            }
            return { type: 'directory', contents: [] };
        },

        // 检查路径是否有效
        isValidPath(path) {
            if (!path.startsWith('/home/')) {
                return false;
            }
            if (!username && !path.startsWith('/home/guest/')) {
                return false;
            }
            if (username && path.startsWith('/home/') && !path.startsWith(`/home/${username}/`)) {
                return false;
            }
            return true;
        }
    };

    // 创建路径显示元素
    function createPathDisplay() {
        if (!pathDisplay) {
            pathDisplay = document.createElement('div');
            pathDisplay.className = 'path-display';
            inputArea.parentNode.insertBefore(pathDisplay, inputArea);
        }
        return pathDisplay;
    }

    // 更新路径显示
    function updatePathDisplay() {
        const display = createPathDisplay();
        display.textContent = currentPath;
    }

    // 创建虚拟文件目录
    async function createVirtualDirectories(username) {
        // 一次性创建所有目录
        const directories = [
            '/home',
            `/home/${username}`,
            `/home/${username}/Desktop`,
            `/home/${username}/Documents`,
            `/home/${username}/Documents/drafts`,
            `/home/${username}/Documents/published`,
            `/home/${username}/Album`,
            `/home/${username}/Album/avatars`,
            `/home/${username}/Album/covers`,
            `/home/${username}/Album/uploads`,
            `/home/${username}/Config`,
            `/home/${username}/Trash`
        ].join(' ');

        // 只发送一次命令
        await sendCommand(`mkdir -p ${directories}`);
    }

    // 检查并创建虚拟文件目录
    async function checkAndCreateVirtualDirectories() {
        if (authToken) {
            // 如果已登录但没有用户名，先获取用户信息
            if (!username) {
                const response = await sendCommand('id');
                if (response.success && response.data && !response.data.is_guest) {
                    username = response.data.username;
                    currentPath = `/home/${username}/`;
                    await createVirtualDirectories(username);
                    updatePathDisplay();
                }
            }
        } else {
            // 未登录状态，设置为访客目录
            username = null;
            currentPath = '/home/guest';
            updatePathDisplay();
        }
    }

    // 处理文件系统命令
    async function handleFileSystemCommand(command) {
        const args = command.split(' ');
        const cmd = args[0];

        switch (cmd) {
            case 'cd':
                if (args.length < 2) {
                    addOutput('用法: cd <目录>', true);
                    return true;
                }
                const newPath = args[1];
                let targetPath;
                
                if (newPath === '~') {
                    targetPath = `/home/${username || 'guest'}/`;
                } else if (newPath === '..') {
                    const parts = currentPath.split('/').filter(Boolean);
                    if (parts.length > 0) {
                        parts.pop();
                        targetPath = '/' + parts.join('/') + (parts.length > 0 ? '/' : '');
                    }
                } else if (newPath.startsWith('/')) {
                    targetPath = newPath.endsWith('/') ? newPath : newPath + '/';
                } else {
                    targetPath = currentPath + newPath + '/';
                }

                const cdResponse = await sendCommand(`cd ${targetPath}`);
                if (cdResponse.success) {
                    currentPath = targetPath;
                    updatePathDisplay();
                } else {
                    addOutput(cdResponse.message, true);
                }
                return true;

            case 'pwd':
                const pwdResponse = await sendCommand('pwd');
                if (pwdResponse.success) {
                    addOutput(pwdResponse.message);
                } else {
                    addOutput(pwdResponse.message, true);
                }
                return true;

            case 'ls':
                const lsPath = args.length > 1 ? args[1] : currentPath;
                const lsResponse = await sendCommand(`ls ${lsPath}`);
                if (lsResponse.success && lsResponse.data) {
                    const contents = lsResponse.data.contents;
                    if (contents.length > 0) {
                        addOutput('目录内容:');
                        contents.forEach(item => {
                            const icon = item.is_directory ? '📁' : '📄';
                            // 使用相对路径显示
                            const relativePath = item.name.startsWith('/') ? 
                                item.name.substring(item.name.lastIndexOf('/') + 1) : 
                                item.name;
                            addOutput(`  ${icon} ${relativePath} ${item.permissions} ${item.owner}`);
                        });
                    } else {
                        addOutput('目录为空');
                    }
                } else {
                    addOutput(lsResponse.message, true);
                }
                return true;

            case 'mkdir':
                if (args.length < 2) {
                    addOutput('用法: mkdir [-p] <目录名>', true);
                    return true;
                }
                const mkdirResponse = await sendCommand(command);
                if (mkdirResponse.success) {
                    addOutput(mkdirResponse.message);
                } else {
                    addOutput(mkdirResponse.message, true);
                }
                return true;

            default:
                return false;
        }
    }

    // 主题切换
    function toggleTheme() {
        const currentTheme = document.body.getAttribute('data-theme');
        const newTheme = currentTheme === 'light' ? 'dark' : 'light';
        document.body.setAttribute('data-theme', newTheme);
        localStorage.setItem('theme', newTheme);

        // 切换图标
        themeIcon.className = newTheme === 'light' ? 'ri-moon-line' : 'ri-sun-line';

        // 更新背景渐变色
        if (newTheme === 'light') {
            document.body.style.background = 'linear-gradient(-45deg, #ffffff, #f5f5f5, #e8e8e8, #f5f5f5)';
        } else {
            document.body.style.background = 'linear-gradient(-45deg, #000000, #0a0a0a, #1a1a1a, #0a0a0a)';
        }
    }

    // 初始化主题
    const savedTheme = localStorage.getItem('theme');
    if (savedTheme) {
        document.body.setAttribute('data-theme', savedTheme);
        themeIcon.className = savedTheme === 'light' ? 'ri-moon-line' : 'ri-sun-line';
        // 初始化背景渐变色
        if (savedTheme === 'light') {
            document.body.style.background = 'linear-gradient(-45deg, #ffffff, #f5f5f5, #e8e8e8, #f5f5f5)';
        } else {
            document.body.style.background = 'linear-gradient(-45deg, #000000, #0a0a0a, #1a1a1a, #0a0a0a)';
        }
    }

    // 主题切换按钮事件监听
    themeToggle.addEventListener('click', toggleTheme);

    // 创建输出区域
    function createOutputArea() {
        if (!outputArea) {
            outputArea = document.createElement('div');
            outputArea.className = 'output-area';
            outputArea.id = 'terminal-output';
            terminalContent.appendChild(outputArea);
            // 触发重排以启动动画
            outputArea.offsetHeight;
            outputArea.classList.add('visible');
        }
        return outputArea;
    }

    // 添加命令输出到终端
    function addOutput(text, isError = false, isCaptcha = false) {
        if (isClearing) return;

        const outputArea = createOutputArea();

        const output = document.createElement('div');
        output.className = `command-output ${isError ? 'error' : 'success'} ${isCaptcha ? 'captcha' : ''}`;
        output.textContent = text;
        outputArea.insertBefore(output, outputArea.firstChild);
        outputArea.scrollTop = 0;

        // 边框高亮动画
        const inputArea = document.querySelector('.input-area');
        if (inputArea) {
            inputArea.classList.remove('success', 'error');
            // 触发重绘以重置动画
            void inputArea.offsetWidth;
            if (isError) {
                inputArea.classList.add('error');
            } else if (!isCaptcha) {
                inputArea.classList.add('success');
            }
        }

        adjustTerminalHeight();
    }

    // 清除输出
    function clearOutput() {
        if (!outputArea) return;

        isClearing = true;
        outputArea.classList.remove('visible');

        // 等待过渡动画完成后再移除元素
        outputArea.addEventListener('transitionend', () => {
            if (outputArea && !outputArea.classList.contains('visible')) {
                outputArea.remove();
                outputArea = null;
                // 重置终端高度
                resetTerminalHeight();
                isClearing = false;
            }
        }, { once: true });
    }

    // 调整终端高度
    function adjustTerminalHeight() {
        if (outputArea) {
            const outputHeight = outputArea.scrollHeight;
            const maxHeight = window.innerHeight * 0.7; // 70vh
            const newHeight = Math.min(outputHeight + 50, maxHeight); // 50px 是输入框的高度
            terminal.style.height = `${newHeight}px`;
        }
    }

    // 重置终端高度
    function resetTerminalHeight() {
        terminal.style.height = 'auto';
    }

    // 生成验证码
    async function generateCaptcha() {
        try {
            const response = await fetch('/api/captcha');
            const data = await response.json();
            if (data.success) {
                return data.data;
            }
            throw new Error(data.message || '获取验证码失败');
        } catch (error) {
            console.error('获取验证码失败:', error);
            throw error;
        }
    }

    // 创建提示容器
    function createHintContainer() {
        if (!hintContainer) {
            hintContainer = document.createElement('div');
            hintContainer.className = 'hint-container';
            // 将提示容器插入到输入框和输出框之间
            if (outputArea) {
                outputArea.parentNode.insertBefore(hintContainer, outputArea);
            } else {
                inputArea.parentNode.insertBefore(hintContainer, inputArea.nextSibling);
            }
        }
        return hintContainer;
    }

    // 显示验证码标签
    async function showCaptchaLabel() {
        try {
            // 确保移除所有现有的验证码标签
            const existingCaptchaLabels = document.querySelectorAll('.captcha-label');
            existingCaptchaLabels.forEach(label => {
                if (label.parentNode) {
                    label.parentNode.removeChild(label);
                }
            });
            captchaLabel = null;

            const captchaData = await generateCaptcha();
            const container = createHintContainer();
            captchaLabel = document.createElement('div');
            captchaLabel.className = 'captcha-label';
            captchaLabel.textContent = `验证码: ${captchaData.captcha}`;
            captchaLabel.dataset.sessionId = captchaData.session_id;
            captchaLabel.style.opacity = '0';
            captchaLabel.style.transform = 'translateY(-10px)';
            container.appendChild(captchaLabel);
            
            // 触发渐进动画
            setTimeout(() => {
                captchaLabel.style.opacity = '1';
                captchaLabel.style.transform = 'translateY(0)';
            }, 10);
        } catch (error) {
            console.error('获取验证码失败:', error);
            addOutput(`错误: ${error.message}`, true);
        }
    }

    // 隐藏验证码标签
    function hideCaptchaLabel() {
        // 移除所有验证码标签
        const existingCaptchaLabels = document.querySelectorAll('.captcha-label');
        existingCaptchaLabels.forEach(label => {
            label.style.opacity = '0';
            label.style.transform = 'translateY(-10px)';
            setTimeout(() => {
                if (label.parentNode) {
                    label.parentNode.removeChild(label);
                }
            }, 300);
        });
        captchaLabel = null;

        // 如果提示容器为空，移除它
        setTimeout(() => {
            if (hintContainer && !hintContainer.hasChildNodes()) {
                hintContainer.parentNode.removeChild(hintContainer);
                hintContainer = null;
            }
        }, 300);
    }

    // 显示命令提示
    function showCommandHint(command) {
        if (!commandHint) {
            const container = createHintContainer();
            commandHint = document.createElement('div');
            commandHint.className = 'command-hint';
            container.appendChild(commandHint);
        }

        const args = command.split(' ');
        let hint = '';

        if (!authToken) {
            // 未登录状态
            switch (args[0]) {
                case 'register':
                    hint = '用法: register <username> <password> --confirm <password> [--captcha <code>] [--show]';
                    break;
                case 'login':
                    hint = '用法: login <username> <password> [--captcha <code>] [--show]';
                    break;
                default:
                    hint = '';
            }
        } else {
            // 已登录状态
            switch (args[0]) {
                case 'logout':
                    hint = '退出当前登录的账号';
                    break;
                case 'profile':
                    hint = '用法: profile --email <email> [--gender <gender>] [--birthday <YYYY-MM-DD>]';
                    break;
                default:
                    hint = '';
            }
        }

        commandHint.textContent = hint;
        commandHint.style.opacity = '0';
        setTimeout(() => {
            commandHint.style.opacity = '0.5';
        }, 10);
    }

    // 隐藏命令提示
    function hideCommandHint() {
        if (commandHint) {
            commandHint.style.opacity = '0';
            setTimeout(() => {
                if (commandHint && commandHint.parentNode) {
                    commandHint.parentNode.removeChild(commandHint);
                    commandHint = null;
                }
                // 如果提示容器为空，移除它
                if (hintContainer && !hintContainer.hasChildNodes()) {
                    hintContainer.parentNode.removeChild(hintContainer);
                    hintContainer = null;
                }
            }, 300);
        }
    }

    // 发送命令到服务器
    async function sendCommand(command, sessionId = '') {
        try {
            const headers = {
                'Content-Type': 'application/json'
            };

            // 如果有 token，添加到请求头
            if (authToken) {
                headers['Authorization'] = `Bearer ${authToken}`;
            }

            const response = await fetch('/api/command', {
                method: 'POST',
                headers: headers,
                body: JSON.stringify({
                    command: command,
                    session_id: sessionId
                })
            });

            const data = await response.json();

            // 如果是登录命令且成功，保存 token 并获取用户信息
            if (command.startsWith('login ') && data.success && data.data && data.data.token) {
                authToken = data.data.token;
                localStorage.setItem('token', authToken);
                
                // 立即发送 id 命令获取用户信息
                const userResponse = await fetch('/api/command', {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                        'Authorization': `Bearer ${authToken}`
                    },
                    body: JSON.stringify({
                        command: 'id',
                        session_id: ''
                    })
                });
                
                const userData = await userResponse.json();
                if (userData.success && userData.data && userData.data.username) {
                    username = userData.data.username;
                    currentPath = `/home/${username}/`;
                    await createVirtualDirectories(username);
                    updatePathDisplay();
                }
            }
            // 如果是登出命令且成功，清除 token 并重置路径
            else if (command === 'logout' && data.success) {
                authToken = null;
                localStorage.removeItem('token');
                username = null;
                currentPath = '/home/guest';
                updatePathDisplay();
            }

            return data;
        } catch (error) {
            console.error('Error:', error);
            return {
                success: false,
                message: '网络错误'
            };
        }
    }

    // 检查并更新虚拟文件系统状态
    async function checkVirtualFileSystem() {
        if (authToken) {
            // 如果已登录但没有用户名，获取用户信息
            if (!username) {
                const response = await sendCommand('id');
                if (response.success && response.data && !response.data.is_guest) {
                    username = response.data.username;
                    currentPath = `/home/${username}/`;
                    updatePathDisplay();
                }
            }
        } else {
            // 未登录状态，设置为访客目录
            username = null;
            currentPath = '/home/guest';
            updatePathDisplay();
        }
    }

    // 修改 handleCommand 函数
    async function handleCommand(command) {
        const args = command.split(' ');
        const cmd = args[0];

        // 先保存 sessionId，再清理提示
        let sessionId = captchaLabel?.dataset.sessionId || '';
        hideCommandHint();

        // 处理文件系统命令
        if (await handleFileSystemCommand(command)) {
            return;
        }

        let response;
        // 处理注册命令
        if (cmd === 'register') {
            // 如果没有验证码，先获取验证码
            if (!captchaLabel) {
                await showCaptchaLabel();
                return;
            }
            response = await sendCommand(command, sessionId);
            if (response.success) {
                hideCaptchaLabel();
                addOutput('注册成功');
            } else {
                addOutput(response.message, true);
            }
            return response;
        }

        // 处理登录命令
        if (cmd === 'login') {
            // 如果登录失败次数达到3次，需要验证码
            if (!captchaLabel) {
                await showCaptchaLabel();
                return;
            }
            
            // 检查命令中是否包含验证码
            const captchaIndex = args.findIndex(arg => arg === '--captcha');
            if (captchaIndex === -1 || captchaIndex + 1 >= args.length) {
                addOutput('请提供验证码: --captcha <code>', true);
                return;
            }
            
            response = await sendCommand(command, sessionId);
            if (response.success) {
                hideCaptchaLabel();
                addOutput('登录成功');
                await checkAndCreateVirtualDirectories();
            } else {
                // 如果登录失败，检查是否需要验证码
                if (response.message.includes('需要验证码')) {
                    await showCaptchaLabel();
                }
                addOutput(response.message, true);
            }
            return response;
        }

        // 处理其他命令
        switch (cmd) {
            case 'id':
                response = await sendCommand(command, sessionId);
                if (response.success) {
                    addOutput(response.message);
                } else {
                    addOutput(response.message, true);
                }
                break;

            case 'profile':
                if (args.length === 1 || (args.length === 2 && args[1] === 'show')) {
                    response = await sendCommand('profile show', sessionId);
                    if (response.success) {
                        addOutput(response.message);
                    } else {
                        addOutput(response.message, true);
                    }
                } else if (args.length >= 3 && args[1] === 'update') {
                    response = await sendCommand(command, sessionId);
                    if (response.success) {
                        addOutput(response.message);
                    } else {
                        addOutput(response.message, true);
                    }
                } else {
                    addOutput('用法: profile show | profile update [--email <email>] [--gender <gender>] [--birthday <YYYY-MM-DD>]', true);
                }
                break;

            case 'help':
                response = await sendCommand(command, sessionId);
                if (response.success) {
                    addOutput(response.message);
                } else {
                    addOutput(response.message, true);
                }
                break;

            case 'clear':
                response = await sendCommand(command, sessionId);
                if (response.success) {
                    clearOutput();
                }
                break;

            case 'logout':
                response = await sendCommand(command, sessionId);
                if (response.success) {
                    hideCaptchaLabel();
                    addOutput('已退出登录');
                    await checkAndCreateVirtualDirectories();
                } else {
                    addOutput(response.message, true);
                }
                break;

            case 'cd':
                response = await sendCommand(command, sessionId);
                if (response.success) {
                    // 更新当前工作目录
                    currentPath = response.data.path;
                    // 更新路径显示
                    updatePathDisplay();
                    addOutput(response.message);
                } else {
                    addOutput(response.message, true);
                }
                break;

            default:
                response = await sendCommand(command, sessionId);
                if (response.success) {
                    addOutput(response.message);
                } else {
                    addOutput(response.message, true);
                }
                break;
        }
        return response;
    }

    // 监听输入框内容变化，动态显示/隐藏验证码和提示
    terminalInput.addEventListener('input', async () => {
        const value = terminalInput.value.trim();
        if (!authToken && (value.startsWith('register') || value.startsWith('login'))) {
            // 如果当前没有验证码，生成新的验证码
            if (!captchaLabel) {
                await showCaptchaLabel();
            }
            showCommandHint(value);
        } else if (authToken && (value.startsWith('logout') || value.startsWith('profile'))) {
            showCommandHint(value);
        } else {
            hideCaptchaLabel();
            hideCommandHint();
        }
    });

    // 处理输入框事件
    terminalInput.addEventListener('keydown', async (e) => {
        if (e.key === 'Enter') {
            const command = terminalInput.value.trim();
            if (command) {
                commandHistory.unshift(command);
                historyIndex = -1;
                await handleCommand(command);
            }
            terminalInput.value = '';
        } else if (e.key === 'ArrowUp') {
            e.preventDefault();
            if (historyIndex < commandHistory.length - 1) {
                historyIndex++;
                terminalInput.value = commandHistory[historyIndex];
            }
        } else if (e.key === 'ArrowDown') {
            e.preventDefault();
            if (historyIndex > 0) {
                historyIndex--;
                terminalInput.value = commandHistory[historyIndex];
            } else if (historyIndex === 0) {
                historyIndex = -1;
                terminalInput.value = '';
            }
        }
    });

    // 初始焦点
    terminalInput.focus();

    // 监听窗口大小变化
    window.addEventListener('resize', () => {
        if (outputArea) {
            adjustTerminalHeight();
        }
    });

    // 自动补全系统
    const CompletionSystem = {
        // 命令补全器
        commandCompleter: {
            // 基础命令列表
            baseCommands: ['ls', 'cd', 'pwd', 'mkdir', 'rm', 'mv', 'cp', 'clear', 'logout', 'id', 'register', 'login'],
            // 博客相关命令
            blogCommands: ['post', 'upload', 'image', 'config', 'profile'],
            // 子命令映射
            subCommands: {
                post: ['create', 'edit', 'publish', 'delete', 'list'],
                image: ['list', 'delete', 'upload'],
                config: ['show', 'set'],
                profile: ['show', 'update']
            }
        },
        
        // 路径补全器
        pathCompleter: {
            // 虚拟文件系统路径
            virtualPaths: {
                '/home': ['guest', '<username>'],
                '/home/<username>': ['Desktop', 'Documents', 'Album', 'Config', 'Trash'],
                '/home/<username>/Documents': ['drafts', 'published'],
                '/home/<username>/Album': ['avatars', 'covers', 'uploads']
            }
        },
        
        // 参数补全器
        argumentCompleter: {
            // 命令参数映射
            commandArgs: {
                'post create': ['--title', '--category', '--tags'],
                'post edit': ['--id', '--title'],
                'config set': ['--theme', '--language', '--timezone'],
                'profile update': ['--email', '--gender', '--birthday'],
                'register': ['--confirm', '--captcha', '--show'],
                'login': ['--captcha', '--show']
            }
        },

        // 获取所有可用命令
        getAllCommands() {
            return [...this.commandCompleter.baseCommands, ...this.commandCompleter.blogCommands];
        },

        // 获取子命令
        getSubCommands(command) {
            return this.commandCompleter.subCommands[command] || [];
        },

        // 获取路径补全选项
        getPathCompletions(path) {
            const normalizedPath = path.endsWith('/') ? path : path + '/';
            return this.pathCompleter.virtualPaths[normalizedPath] || [];
        },

        // 获取参数补全选项
        getArgumentCompletions(command) {
            return this.argumentCompleter.commandArgs[command] || [];
        },

        // 执行补全
        complete(input) {
            const parts = input.split(' ');
            const currentPart = parts[parts.length - 1];
            const previousPart = parts[parts.length - 2];

            // 如果是第一个部分，补全命令
            if (parts.length === 1) {
                let commands = this.getAllCommands();
                
                // 根据登录状态过滤命令
                if (authToken) {
                    // 已登录状态，移除 login 和 register
                    commands = commands.filter(cmd => !['login', 'register'].includes(cmd));
                } else {
                    // 未登录状态，移除 logout 和 profile
                    commands = commands.filter(cmd => !['logout', 'profile'].includes(cmd));
                }
                
                return commands.filter(cmd => cmd.startsWith(currentPart));
            }

            // 如果是第二个部分，补全子命令
            if (parts.length === 2) {
                const subCommands = this.getSubCommands(parts[0]);
                return subCommands.filter(cmd => cmd.startsWith(currentPart));
            }

            // 如果是路径，补全路径
            if (currentPart.startsWith('/') || currentPart.startsWith('./')) {
                return this.getPathCompletions(currentPart).filter(path => path.startsWith(currentPart));
            }

            // 如果是参数，补全参数
            let command = parts[0];
            if (parts.length >= 2) {
                command = `${parts[0]} ${parts[1]}`;
            }
            const args = this.getArgumentCompletions(command);
            
            // 过滤掉已经使用过的参数
            const usedArgs = parts.filter(part => part.startsWith('--'));
            return args.filter(arg => !usedArgs.includes(arg) && arg.startsWith(currentPart));
        }
    };

    // 处理 Tab 补全
    function handleTabCompletion(e) {
        if (e.key === 'Tab') {
            e.preventDefault();
            const input = terminalInput.value;
            const cursorPosition = terminalInput.selectionStart;
            const textBeforeCursor = input.substring(0, cursorPosition);
            
            // 获取补全选项
            const completions = CompletionSystem.complete(textBeforeCursor);
            
            if (completions.length === 1) {
                // 如果只有一个选项，直接补全
                const completion = completions[0];
                const parts = textBeforeCursor.split(' ');
                parts[parts.length - 1] = completion;
                terminalInput.value = parts.join(' ') + ' ';
                terminalInput.selectionStart = terminalInput.selectionEnd = terminalInput.value.length;
            } else if (completions.length > 1) {
                // 如果有多个选项，显示所有选项
                addOutput('可用的补全选项:');
                completions.forEach(completion => {
                    addOutput(`  ${completion}`);
                });
            }
        }
    }

    // 添加 Tab 补全事件监听
    terminalInput.addEventListener('keydown', handleTabCompletion);
});