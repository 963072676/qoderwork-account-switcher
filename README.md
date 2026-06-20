# QoderWork Account Switcher

<div align="center">

[![Build Status](https://img.shields.io/github/actions/workflow/status/963072676/qoderwork-account-switcher/release.yml?logo=github&label=构建)](https://github.com/963072676/qoderwork-account-switcher/actions)
[![License](https://img.shields.io/github/license/963072676/qoderwork-account-switcher?label=许可证)](./LICENSE)
[![Release](https://img.shields.io/github/v/release/963072676/qoderwork-account-switcher?label=最新版本)](https://github.com/963072676/qoderwork-account-switcher/releases/latest)

**一键切换 QoderWork CN 账号，轻松管理多账号会话、额度与签到**

[下载安装](#安装) · [使用指南](#使用指南) · [开发指南](#开发指南) · [问题反馈](https://github.com/963072676/qoderwork-account-switcher/issues)

</div>

---

## 截图

![screenshot](docs/screenshot.png)

---

## 功能特性

- **一键切换账号** — 无需重复登录，一键即可切换至目标 QoderWork CN 账号
- **多账号管理** — 支持同时管理 N 个账号，自由添加、删除和重命名
- **实时额度展示** — 四列显示每个账号的核心额度信息：
  - 每日免费 Qwen 3.7 Max 模型剩余额度
  - 计划额度 + 个人资源包 + 企业资源包总剩余
  - 当日签到状态
  - 订阅剩余天数
- **一键签到** — 为所有已保存账号批量领取每日签到奖励，签到后自动刷新区额信息
- **自动检测安装路径** — 智能识别 QoderWork CN 的安装位置，支持手动浏览选择
- **自动保存会话数据** — 每个账号的会话信息独立保存，切换时自动恢复
- **安全加密存储** — 采用 Electron safeStorage (AES-256-GCM) + DPAPI 加密保护认证数据
- **跨平台支持** — 同时支持 Windows 和 macOS 系统

---

## 安装

前往 [Releases 页面](https://github.com/963072676/qoderwork-account-switcher/releases/latest) 下载最新版本的安装包：

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
2. **设置路径** — 点击右上角「设置」，配置 QoderWork CN 的可执行文件路径（支持自动检测和手动浏览选择）
3. **添加账号** — 点击「添加账号」按钮，输入手机号和备注名称进行标识
4. **保存会话** — 在当前 QoderWork CN 登录后，点击「保存当前」捕获登录状态
5. **一键切换** — 在账号列表中选择目标账号，点击「切换」即可

### 功能说明

#### 额度信息

每个账号自动展示四列关键信息：

- **每日免费** — Qwen 3.7 Max 模型的每日免费使用剩余次数（通过 Activity API 实时获取）
- **其他额度** — 计划额度、个人资源包、企业资源包的总剩余额度
- **签到状态** — 当日是否已完成签到（绿色对勾表示已签到）
- **订阅天数** — 当前订阅计划距到期的剩余天数

额度数据在添加账号后自动获取，也可通过页面底部按钮手动刷新。

#### 一键签到

点击底部的「一键签到」按钮，系统会自动为所有已保存的账号执行签到操作：

- 并行调用所有账号的签到领取接口
- 显示签到结果摘要（成功数 / 已签到数 / 失败数）
- 签到完成后自动刷新所有账号的额度和签到状态

#### 添加账号

点击主界面的「添加账号」按钮，输入手机号和备注名称。应用会自动检测当前 QoderWork CN 的会话状态并关联到该账号。

#### 保存会话

在 QoderWork CN 中完成登录后，回到本应用点击「保存当前」。应用会自动捕获并加密保存当前的认证信息，以便后续快速切换。

#### 切换账号

在账号列表中选择想要切换的目标账号，点击「切换」按钮。应用会自动替换 QoderWork CN 的会话数据，完成账号切换。

#### 设置

点击右上角齿轮图标打开设置面板：

- **程序路径** — 支持自动检测和手动浏览选择 QoderWork CN 的可执行文件路径
- 路径配置会自动持久化保存，无需每次重新设置

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
git clone https://github.com/963072676/qoderwork-account-switcher.git
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
├── src/                      # 前端源码 (React + TypeScript)
│   ├── components/           # UI 组件
│   │   ├── AccountList.tsx   # 账号列表（含额度展示）
│   │   ├── AccountForm.tsx   # 添加账号表单
│   │   ├── Header.tsx        # 顶部导航栏
│   │   ├── SettingsModal.tsx # 设置弹窗
│   │   └── SwitchProgress.tsx# 切换进度条
│   ├── hooks/
│   │   └── useAccounts.ts    # 账号管理核心 Hook
│   ├── types.ts              # TypeScript 类型定义
│   └── App.tsx               # 应用入口
├── src-tauri/                # Tauri / Rust 后端
│   ├── src/
│   │   ├── lib.rs            # 插件注册与命令入口
│   │   ├── commands/         # Tauri 命令模块
│   │   │   ├── accounts.rs   # 账号 CRUD
│   │   │   ├── switch.rs     # 账号切换逻辑
│   │   │   ├── detect.rs     # 路径检测与设置持久化
│   │   │   └── quota_cmd.rs  # 额度查询与签到命令
│   │   ├── core/             # 核心业务逻辑
│   │   │   ├── quota.rs      # 额度/签到/Cosy认证
│   │   │   ├── paths.rs      # 路径管理
│   │   │   └── state.rs      # 状态持久化
│   │   └── error.rs          # 统一错误处理
│   ├── capabilities/         # Tauri v2 权限配置
│   ├── Cargo.toml            # Rust 依赖配置
│   └── tauri.conf.json       # Tauri 配置
├── public/                   # 静态资源
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
| [tauri-plugin-store](https://github.com/tauri-apps/plugins-workspace) | 设置持久化 |
| [tauri-plugin-dialog](https://github.com/tauri-apps/plugins-workspace) | 文件对话框 |

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
