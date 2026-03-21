# 项目结构

## 前端目录

- `src/App.vue`：应用壳层入口
- `src/bootstrap.ts`：应用启动初始化
- `src/main.ts`：前端挂载入口
- `src/router`：路由定义
- `src/navigation`：菜单与导航配置
- `src/layouts`：通用布局组件
- `src/styles.css`：全局样式
- `src/modules/download`：视频下载模块
- `src/modules/tasks`：任务记录、批次明细、任务明细
- `src/modules/settings`：设置页面、设置状态与设置 API
- `src/modules/recording`：直播录制模块
- `src/api`：跨模块通用 Tauri API 封装
- `src/i18n`：国际化词典与初始化
- `src/theme`：主题运行时逻辑
- `src/stores`：全局状态
- `src/lib`：通用工具与错误处理
- `src/assets`：静态资源

## Rust 目录

- `src-tauri/src/application`：应用启动状态与上下文
- `src-tauri/src/commands`：对外 Tauri 命令面
- `src-tauri/src/domain`：领域模型
- `src-tauri/src/repositories`：数据库访问
- `src-tauri/src/services`：队列管理、Python 桥接等服务

## Python 目录

- `src-tauri/python/core`：协议分发与运行流程
- `src-tauri/python/tasks`：具体任务实现
- `src-tauri/python/main.py`：轻量入口

## 维护规则

- 新业务优先落在对应的 `src/modules` 模块目录
- 只有在确认可复用后，才抽到 `src/api`、`src/lib`、`src/stores`、`src/theme` 等通用目录
- 不要重新引入 `src/app`、`src/shared` 这一类额外套层
- 优先保持文件职责单一，避免把页面、状态、API、类型堆进同一个文件
