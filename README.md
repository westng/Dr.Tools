<p align="center">
  <img src="https://avatars.githubusercontent.com/u/277389313?s=200&v=4" width="128" height="128" alt="Dr.Tools">
</p>

<h1 align="center">Dr.Tools</h1>

<p align="center">
  面向创作者媒体工作流的桌面工具箱。
</p>

<p align="center">
  桌面应用 · 视频下载 · 任务管理 · 设置管理 · Tauri 集成
</p>

<p align="center">
  <a href="package.json"><img src="https://img.shields.io/badge/Desktop-App-4F46E5" alt="Desktop App"></a>
  <a href="src-tauri/Cargo.toml"><img src="https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri" alt="Tauri 2"></a>
  <a href="package.json"><img src="https://img.shields.io/badge/Stack-Vue%203%20%2B%20Rust-42B883?logo=vue.js" alt="Vue 3 and Rust"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-green.svg" alt="License"></a>
</p>

<p align="center">
  简体中文 | <a href="README.en.md">English</a>
</p>

Dr.Tools 是一个基于 Tauri、Vue、Rust、SQLite 和 Python 的桌面应用。
当前主要聚焦于创作者媒体工作流，包括视频下载、任务记录和运行设置管理。

> 说明：项目仍处于开发阶段，功能与实现会持续调整。
>
> 项目中的视频下载与直播录制能力基于开源项目 [`F2`](https://github.com/Johnserf-Seed/f2) 进行集成与扩展。这里明确声明依赖来源，是为了遵循开源精神，尊重原项目工作，并尽量避免不必要的版权与归属争议。

## 功能特性

- 视频下载提交与批次队列处理
- 单任务处理日志展示
- 任务记录、批次明细、任务明细窗口
- 主题、语言、Token、下载设置管理
- Python sidecar 任务执行桥接
- 批次完成后的系统通知

## 技术栈

- Vue 3
- TypeScript
- Pinia
- Vue Router
- Vite
- Tauri 2
- Rust
- SQLite
- Python
- f2

## 开发使用

### 环境要求

- Node.js 20+
- `pnpm`
- Rust 工具链
- Python 3.12

### 安装依赖

```bash
pnpm install
```

### 启动开发环境

```bash
pnpm tauri dev
```

### 校验命令

```bash
pnpm typecheck
pnpm build
pnpm check:desktop
pnpm check
```

## 构建与发版

- 已新增 GitHub Actions 工作流：`.github/workflows/release.yml`
- 手动构建：在 GitHub 仓库的 Actions 页面运行 `release` 工作流，可生成各平台构建产物并作为 workflow artifacts 上传
- 自动发版：推送 `v*` 格式标签，例如 `v0.1.0`，会自动构建并创建 GitHub Release，上传对应安装包
- 当前工作流覆盖平台：macOS Apple Silicon、macOS Intel、Linux、Windows
- 如需启用正式签名，后续可在仓库 Secrets 中补充 macOS / Windows 的签名凭据

## 设计系统预览

- 设计系统文档：`DESIGN_SYSTEM.md`
- 静态预览页：`design-system-preview.html`
- 使用方式：直接在浏览器中打开根目录 `design-system-preview.html`
- 维护要求：设计规则变更时，必须同步更新 `DESIGN_SYSTEM.md` 与 `design-system-preview.html`

## 项目结构

```text
src/                         前端应用入口与界面层
  App.vue                    应用壳与通用标题栏
  main.ts                    前端启动入口
  bootstrap.ts               应用初始化与全局错误接管
  router/                    路由注册与页面入口
  layouts/                   通用布局组件
  navigation/                导航配置
  modules/                   按业务拆分的前端模块
    download/                视频下载
    recording/               直播录制
    tasks/                   任务记录与明细
    settings/                设置与运行配置
  api/                       通用 Tauri API 封装
  stores/                    全局状态
  i18n/                      多语言文案
  theme/                     外观与主题逻辑
  lib/                       通用工具函数
  assets/                    静态资源

src-tauri/                   桌面端宿主与后端能力
  src/
    main.rs                  Tauri 应用入口
    application/             应用状态装配与启动逻辑
    commands/                对前端暴露的 Tauri commands
    domain/                  领域模型与类型
    repositories/            SQLite 数据访问
    services/                调度、Python 桥接与系统服务
  python/                    Python sidecar 入口与任务实现
    core/                    Python 任务分发与基础设施
    tasks/                   Python 具体任务
  migrations/                SQLite 迁移脚本
```

## 说明

- 前端优先按业务模块组织，页面、类型、接口尽量收敛在 `src/modules/<业务>` 内。
- 跨模块复用能力统一下沉到 `src/api`、`src/i18n`、`src/lib`、`src/stores`、`src/theme` 与 `src/assets`。
- Rust 侧按 `commands -> services -> repositories -> domain` 的职责链拆分，避免 command 直接堆业务细节。
- Python `main.py` 只负责协议入口，具体任务分发与实现分别放在 `src-tauri/python/core` 和 `src-tauri/python/tasks`。

## 界面预览

![Dr.Tools Screenshot 1](screenshots/ScreenShot_2026-03-22_184248_670.png)
![Dr.Tools Screenshot 2](screenshots/ScreenShot_2026-03-22_184312_268.png)
![Dr.Tools Screenshot 3](screenshots/ScreenShot_2026-03-22_184850_010.png)

## 参与贡献

提交代码前请先阅读 `CONTRIBUTING.md`。

## 鸣谢

- 感谢 [`F2`](https://github.com/Johnserf-Seed/f2) 为项目提供支持。

## 开源协议

项目整体采用 `MIT` 开源协议，详见 `LICENSE`。
