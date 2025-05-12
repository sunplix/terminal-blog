# 终端博客系统 (Terminal Blog System)

一个基于终端操作风格的博客系统，通过模拟Linux终端命令来实现博客的创建、管理和访问。

## 项目特点

- 🖥️ 终端风格界面：提供类Linux终端的操作体验
- 📝 博客功能：支持文章的创建、编辑、发布和管理
- 🔐 用户系统：完整的用户认证和权限管理
- 🗂️ 虚拟文件系统：模拟Linux文件系统，管理博客内容
- 🎨 响应式设计：支持多种设备访问

## 功能特性

### 用户系统
- 用户注册与登录
- 验证码支持
- 用户信息管理
- 权限控制（管理员/作者/访客）

### 文件系统
- 目录操作：`cd`、`ls`、`pwd`、`mkdir`等
- 文件操作：创建、编辑、删除
- 权限管理：基于Unix风格的权限控制

### 博客功能
- 文章管理：创建、编辑、删除
- 草稿系统：支持文章草稿
- 分类管理：通过目录结构组织文章
- 媒体管理：支持图片等媒体文件

### 安全特性
- JWT认证
- 密码加密存储
- 登录尝试限制
- Token黑名单

## 技术架构

### 后端
- 语言：Rust
- Web框架：Actix-web
- 数据库：PostgreSQL
- 认证：JWT
- 文件系统：自定义VFS实现

### 前端
- 纯JavaScript实现
- 响应式设计
- 终端模拟器
- 命令补全

## 系统要求

- Rust 1.70+
- PostgreSQL 12+
- Node.js 14+ (用于开发)

## 安装部署

### 1. 克隆项目
```bash
git clone https://github.com/yourusername/terminal-blog.git
cd terminal-blog
```

### 2. 配置环境
创建`.env`文件：
```env
DATABASE_URL=postgres://username:password@localhost/terminal_blog
JWT_SECRET=your_jwt_secret
RUST_LOG=info
```

### 3. 初始化数据库
```bash
sqlx database create
sqlx migrate run
```

### 4. 编译运行
```bash
cargo build --release
cargo run --release
```

## 使用说明

### 基本命令
- `register` - 注册新用户
- `login` - 用户登录
- `logout` - 退出登录
- `id` - 显示当前用户信息
- `profile` - 管理用户资料

### 文件系统命令
- `cd` - 切换目录
- `ls` - 列出目录内容
- `pwd` - 显示当前目录
- `mkdir` - 创建目录

### 博客操作
- 文章创建：在`Documents/drafts`目录下创建文件
- 文章发布：将文件移动到`Documents/published`目录
- 媒体管理：在`Album`目录下管理图片等媒体文件

## 目录结构
```
/home/
  ├── username/
  │   ├── Desktop/
  │   ├── Documents/
  │   │   ├── drafts/      # 草稿目录
  │   │   └── published/   # 已发布文章
  │   ├── Album/
  │   │   ├── avatars/     # 头像
  │   │   ├── covers/      # 封面图
  │   │   └── uploads/     # 上传文件
  │   ├── Config/          # 配置文件
  │   └── Trash/           # 回收站
```

## 开发说明

### 项目结构
```
src/
  ├── auth/         # 认证模块
  ├── command/      # 命令处理
  ├── vfs/          # 虚拟文件系统
  ├── captcha/      # 验证码
  └── main.rs       # 入口文件
```

### 添加新命令
1. 在`src/command`目录下创建新命令文件
2. 实现`CommandHandler` trait
3. 在`mod.rs`中注册命令

## 贡献指南

1. Fork 项目
2. 创建特性分支
3. 提交更改
4. 推送到分支
5. 创建 Pull Request

## 许可证

MIT License

## 作者

[Your Name]

## 致谢

- Actix-web
- SQLx
- JWT
- 其他开源项目 