document.addEventListener('DOMContentLoaded', () => {
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
                if (newPath === '~') {
                    currentPath = `/home/${username}/`;
                } else if (newPath === '..') {
                    // 处理返回上级目录
                    const parts = currentPath.split('/').filter(Boolean);
                    if (parts.length > 0) {
                        parts.pop();
                        currentPath = '/' + parts.join('/') + (parts.length > 0 ? '/' : '');
                    }
                } else if (newPath.startsWith('/')) {
                    // 处理绝对路径
                    currentPath = newPath.endsWith('/') ? newPath : newPath + '/';
                } else {
                    // 处理相对路径
                    currentPath = currentPath + newPath + '/';
                }
                updatePathDisplay();
                return true;

            case 'pwd':
                addOutput(currentPath);
                return true;

            case 'ls':
                // TODO: 实现目录列表功能
                addOutput('目录列表功能待实现');
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
        // 将新输出插入到输出区域的开头
        outputArea.insertBefore(output, outputArea.firstChild);
        outputArea.scrollTop = 0;

        // 调整终端高度
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
        // 如果已经有验证码标签，先移除它
        if (captchaLabel) {
            hideCaptchaLabel();
        }

        try {
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
        if (captchaLabel) {
            captchaLabel.style.opacity = '0';
            captchaLabel.style.transform = 'translateY(-10px)';
            setTimeout(() => {
                if (captchaLabel && captchaLabel.parentNode) {
                    captchaLabel.parentNode.removeChild(captchaLabel);
                    captchaLabel = null;
                }
                // 如果提示容器为空，移除它
                if (hintContainer && !hintContainer.hasChildNodes()) {
                    hintContainer.parentNode.removeChild(hintContainer);
                    hintContainer = null;
                }
            }, 300);
        }
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

            // 如果是登录命令且成功，保存 token
            if (command.startsWith('login ') && data.success && data.data && data.data.token) {
                authToken = data.data.token;
                localStorage.setItem('token', authToken);
            }
            // 如果是登出命令且成功，清除 token
            else if (command === 'logout' && data.success) {
                authToken = null;
                localStorage.removeItem('token');
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

    // 处理命令输入
    async function handleCommand(command) {
        if (!command.trim()) return;

        // 处理本地命令
        if (command.trim() === 'clear') {
            if (outputArea) {
                addOutput(`$ ${command}`);
            }
            clearOutput();
            return;
        }

        // 隐藏命令提示和验证码
        hideCommandHint();
        hideCaptchaLabel();

        // 显示用户输入（处理密码隐藏）
        let displayCommand = command;
        const args = command.split(' ');

        // 处理注册命令的密码显示
        if (args[0] === 'register' && args.length >= 3) {
            const showPassword = args.includes('--show');
            if (!showPassword) {
                displayCommand = `${args[0]} ${args[1]} ${'*'.repeat(args[2].length)}`;
                const confirmIndex = args.indexOf('--confirm');
                if (confirmIndex !== -1 && confirmIndex + 1 < args.length) {
                    displayCommand += ` --confirm ${'*'.repeat(args[confirmIndex + 1].length)}`;
                }
            }
        }
        // 处理登录命令的密码显示
        else if (args[0] === 'login' && args.length >= 3) {
            const showPassword = args.includes('--show');
            if (!showPassword) {
                displayCommand = `${args[0]} ${args[1]} ${'*'.repeat(args[2].length)}`;
            }
        }

        addOutput(`$ ${displayCommand}`);

        try {
            // 处理文件系统命令
            if (await handleFileSystemCommand(command)) {
                return;
            }

            // 验证注册命令的必选参数
            if (args[0] === 'register') {
                // 检查是否包含确认密码
                const confirmIndex = args.indexOf('--confirm');
                if (confirmIndex === -1 || confirmIndex + 1 >= args.length) {
                    addOutput('错误: 注册时必须使用 --confirm 参数确认密码', true);
                    return;
                }
                // 检查密码是否匹配
                if (args[2] !== args[confirmIndex + 1]) {
                    addOutput('错误: 两次输入的密码不匹配', true);
                    return;
                }
            }

            let sessionId = '';
            let captchaCode = '';

            // 如果是注册或登录命令且包含验证码，验证验证码
            if ((args[0] === 'register' || args[0] === 'login') && args.includes('--captcha')) {
                const captchaIndex = args.indexOf('--captcha');
                if (captchaIndex + 1 >= args.length) {
                    addOutput('错误: 请提供验证码', true);
                    return;
                }
                captchaCode = args[captchaIndex + 1];
                sessionId = captchaLabel?.dataset.sessionId || '';
            }
            // 如果是注册或登录命令但没有包含验证码，使用已显示的验证码
            else if ((args[0] === 'register' || args[0] === 'login') && !args.includes('--captcha')) {
                if (!captchaLabel) {
                    addOutput('错误: 请先输入用户名和密码', true);
                    return;
                }
                sessionId = captchaLabel.dataset.sessionId;
                // 不再弹出提示框，而是提示用户在输入框下方查看验证码
                addOutput('请查看输入框下方的验证码', true);
                return;
            }

            const data = await sendCommand(command, sessionId);
            if (data.message) {
                // 如果是登录命令且成功，设置用户名和路径
                if (command.startsWith('login ') && data.success && data.data && data.data.username) {
                    username = data.data.username;
                    currentPath = `/home/${username}/`;
                    updatePathDisplay();
                }
                // 如果是登出命令且成功，重置路径
                else if (command === 'logout' && data.success) {
                    username = null;
                    currentPath = '/';
                    updatePathDisplay();
                }

                // 如果是 id 命令且成功，显示用户详细信息
                if (command === 'id' && data.success && data.data) {
                    const userData = data.data;
                    if (userData.is_guest) {
                        addOutput('当前为访客模式');
                    } else {
                        // 如果是已登录用户，更新用户名和路径
                        if (userData.username && !username) {
                            username = userData.username;
                            currentPath = `/home/${username}/`;
                            updatePathDisplay();
                        }
                        let userInfo = `用户ID: ${userData.id}\n用户名: ${userData.username}`;
                        // 使用淡灰色显示未设置的信息
                        userInfo += `\n邮箱: ${userData.email || '未设置'}`;
                        userInfo += `\n性别: ${userData.gender || '未设置'}`;
                        userInfo += `\n生日: ${userData.birthday || '未设置'}`;

                        // 创建带有样式的输出
                        const output = document.createElement('div');
                        output.className = 'command-output success';

                        // 将文本按行分割并添加样式
                        const lines = userInfo.split('\n');
                        lines.forEach((line, index) => {
                            const lineSpan = document.createElement('div');
                            // 为未设置的信息添加淡灰色样式
                            if (index >= 2 && line.endsWith('未设置')) {
                                lineSpan.style.color = '#888';
                            }
                            lineSpan.textContent = line;
                            output.appendChild(lineSpan);
                        });

                        // 将新输出插入到输出区域的开头
                        if (outputArea) {
                            outputArea.insertBefore(output, outputArea.firstChild);
                            outputArea.scrollTop = 0;
                            adjustTerminalHeight();
                        }
                    }
                } else {
                    // 显示命令执行结果消息
                    addOutput(data.message, !data.success);
                }

                // 如果登录失败次数过多，需要验证码
                if (!data.success && data.message.includes('登录失败次数过多')) {
                    await showCaptchaLabel();
                }
            }
        } catch (error) {
            addOutput('命令执行出错: ' + error.message, true);
        }
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
});