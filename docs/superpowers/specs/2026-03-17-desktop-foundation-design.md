# Dr.Tools 桌面基础设计

- 日期：2026-03-17
- 范围：仅第一阶段基础框架

## 目标

构建一个跨平台桌面基础工程（Windows + macOS），包含：

- Tauri 2 + Vue 3 + TypeScript + Vue Router + Pinia
- Rust 命令层
- SQLite 本地数据库
- 内嵌 Python 执行链路（接口优先）

## 架构

- 前端只调用 Rust 命令。
- Rust 负责数据库访问与 sidecar 生命周期。
- Python 负责任务执行并返回结构化结果。

## 模块边界

- `src/`：UI 壳层、路由、导航、模块、状态、前端服务
- `src-tauri/src/commands/`：Tauri 命令 API
- `src-tauri/src/repositories/`：SQLite 仓储实现
- `src-tauri/src/services/`：队列管理与 Python 桥接
- `src-tauri/python/`：运行时脚本与任务处理
- `src-tauri/migrations/`：数据库迁移文件

## 数据流

1. 前端调用任务提交命令。
2. Rust 校验输入并写入任务记录，状态初始为 `queued`。
3. Rust 将任务派发给 Python sidecar。
4. Rust 更新任务状态并写入任务日志。
5. 前端通过任务列表接口刷新视图。

## 初始表结构

- `app_meta(key, value, updated_at)`
- `tasks(id, task_type, status, input_json, output_json, error_text, created_at, updated_at)`
- `task_logs(id, task_id, level, message, ts)`

## 错误约定

- `VALIDATION_ERROR`
- `PYTHON_START_ERROR`
- `TASK_EXEC_ERROR`
- `DB_ERROR`
