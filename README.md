# QoderWork Account Switcher

<div align="center">

[![Build Status](https://img.shields.io/github/actions/workflow/status/your-username/qoderwork-account-switcher/release.yml?logo=github&label=构建)](https://github.com/your-username/qoderwork-account-switcher/actions)
[![License](https://img.shields.io/github/license/your-username/qoderwork-account-switcher?label=许可证)](./LICENSE)
[![Release](https://img.shields.io/github/v/release/your-username/qoderwork-account-switcher?label=最新版本)](https://github.com/your-username/qoderwork-account-switcher/releases/latest)

**一键切换 QoderWork CN 账号，轻松管理多账号会话**

[下载安装](#安装) · [使用指南](#使用指南) · [开发指南](#开发指南) · [问题反馈](https://github.com/your-username/qoderwork-account-switcher/issues)

</div>

---

## 截图

![screenshot](docs/screenshot.png)

---

## 功能特性

- **一键切换账号** — 无需重复登录，一键即可切换至目标 QoderWork CN 账号
- **多账号管理** — 支持同时管理 N 个账号，自由添加、删除和重命名
- **自动检测安装路径** — 智能识别 QoderWork CN 的安装位置，无需手动配置
- **自动保存会话数据** — 每个账号的会话信息独立保存，切换时自动恢复
- **跨平台支持** — 同时支持 Windows 和 macOS 系统

---

## 安装

前往 [Releases 页面](https://github.com/your-username/qoderwork-account-switcher/releases/latest) 下载最新版本的安装包：

| 操作系统 | 文件格式 | 说明 |
| --- | --- | --- |
| Windows | `.exe` | NSIS 安装程序，双击运行即可 |
| Windows | `.msi` | Windows Installer 格式 |
| macOS (Apple Silicon) | `.dmg` | 双击打开，将应用拖入 Applications |
| macOS (Apple Silicon) | `.app.tar.gz` | 解压后直接使用 |

---

## 使用指南

### 首次使用

1. **启动应用** — 安装并打开 QoderWork Account Switcher
2. **添加账号** — 点击「添加账号」按钮，输入账号名称进行标识
3. **保存会话** — 在当前 QoderWork CN 登录后，点击「保存当前会话」捕获登录状态
4. **一键切换** — 在账号列表中选择目标账号，点击「切换」即可

### 功能说明

#### 添加账号

点击主界面的「添加账号」按钮，为该账号设置一个易于识别的名称。应用会自动检测当前 QoderWork CN 的会话状态并关联到该账号。

#### 保存会话

在 QoderWork CN 中完成登录后，回到本应用点击「保存当前会话」。应用会自动捕获并保存当前的认证信息，以便后续快速切换。

#### 切换账号

在账号列表中选择想要切换的目标账号，点击「切换」按钮。应用会自动替换 QoderWork CN 的会话数据，完成账号切换。

#### 管理账号

- **重命名**：右键点击账号条目，选择「重命名」
- **删除**：右键点击账号条目，选择「删除」
- **刷新状态**：点击账号条目右侧的刷新图标，检测当前会话是否仍然有效

---

## 开发指南

### 环境要求

在开始开发之前，请确保您的系统已安装以下工具：

- [Rust](https://www.rust-lang.org/tools/install) (最新稳定版)
- [Node.js](https://nodejs.org/) 20.x 或更高版本
- [pnpm](https://pnpm.io/installation) 9.x 或更高版本
- Tauri v2 平台依赖：
  - **Windows**: [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)（Windows 10/11 已预装）+ Visual Studio Build Tools
  - **macOS**: Xcode Command Line Tools（`xcode-select --install`）

### 本地开发

```bash
# 克隆仓库
git clone https://github.com/your-username/qoderwork-account-switcher.git
cd qoderwork-account-switcher

# 安装依赖
pnpm install

# 启动开发服务器（前端 + Tauri 窗口）
pnpm tauri dev

# 仅启动前端开发服务器（不启动 Tauri）
pnpm dev
```

### 构建发布版本

```bash
# 构建生产版本
pnpm tauri build
```

构建产物位于 `src-tauri/target/release/bundle/` 目录下：
- Windows: `nsis/*.exe` 和 `msi/*.msi`
- macOS: `dmg/*.dmg` 和 `macos/*.app`

### 项目结构

```
qoderwork-account-switcher/
├── src/                  # 前端源码 (React + TypeScript)
│   ├── components/       # UI 组件
│   ├── hooks/            # 自定义 Hooks
│   ├── lib/              # 工具函数
│   └── App.tsx           # 应用入口
├── src-tauri/            # Tauri / Rust 后端
│   ├── src/              # Rust 源码
│   │   └── main.rs       # 后端入口
│   ├── Cargo.toml        # Rust 依赖配置
│   └── tauri.conf.json   # Tauri 配置
├── public/               # 静态资源
├── package.json
├── tsconfig.json
├── tailwind.config.js
└── vite.config.ts
```

---

## 技术栈

| 技术 | 说明 |
| --- | --- |
| [Tauri v2](https://v2.tauri.app/) | 桌面应用框架 |
| [React](https://react.dev/) | 前端 UI 库 |
| [TypeScript](https://www.typescriptlang.org/) | 类型安全 |
| [Tailwind CSS](https://tailwindcss.com/) | 原子化 CSS 框架 |
| [Rust](https://www.rust-lang.org/) | 后端逻辑 |

---

## 贡献指南

欢迎提交 Issue 和 Pull Request！

1. Fork 本仓库
2. 创建功能分支 (`git checkout -b feature/your-feature`)
3. 提交更改 (`git commit -m 'feat: 添加某功能'`)
4. 推送分支 (`git push origin feature/your-feature`)
5. 创建 Pull Request

---

## 许可证

本项目基于 [MIT 许可证](./LICENSE) 开源。

Copyright (c) 2026 王皓晨 (Wang Haochen)
