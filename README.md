# Dr.Tools

Dr.Tools 是一个基于 Tauri、Vue、Rust、SQLite 和 Python 的桌面应用。
当前主要聚焦于创作者媒体工作流，包括视频下载、任务记录和运行设置管理。

> 说明：项目仍处于开发阶段，功能与实现会持续调整。
>
> 项目中的视频下载与直播录制能力基于开源项目 `F2` 进行集成与扩展。这里明确声明依赖来源，是为了遵循开源精神，尊重原项目工作，并尽量避免不必要的版权与归属争议。

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

## 设计系统预览

- 设计系统文档：`DESIGN_SYSTEM.md`
- 静态预览页：`design-system-preview.html`
- 使用方式：直接在浏览器中打开根目录 `design-system-preview.html`
- 维护要求：设计规则变更时，必须同步更新 `DESIGN_SYSTEM.md` 与 `design-system-preview.html`

## 项目结构

```text
src/
  App.vue
  bootstrap.ts
  main.ts
  router/
  navigation/
  layouts/
  modules/
  api/
  assets/
  i18n/
  lib/
  stores/
  theme/

src-tauri/
  src/
    application/
    commands/
    domain/
    repositories/
    services/
  python/
    core/
    tasks/
  migrations/
```

## 说明

- 前端业务代码统一放在 `src/modules`
- 通用前端能力放在 `src/api`、`src/i18n`、`src/lib`、`src/stores`、`src/theme`、`src/assets`
- Tauri command 保持外部调用稳定，内部 Rust 代码按职责拆分
- Python `main.py` 只负责协议入口，任务分发放在 `src-tauri/python/core`

## 参与贡献

提交代码前请先阅读 `CONTRIBUTING.md`。

## 开源协议

项目采用 `MIT` 开源协议，详见 `LICENSE`。
