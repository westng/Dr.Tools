# 架构概览

## 前端

前端按职责拆分为几类目录：

- `src/App.vue`、`src/bootstrap.ts`、`src/main.ts`：应用入口与启动流程
- `src/router`、`src/navigation`、`src/layouts`：路由、一级导航与壳层布局
- `src/modules`：业务模块，当前包含视频下载、任务记录、设置、直播录制
- `src/api`、`src/i18n`、`src/lib`、`src/stores`、`src/theme`、`src/assets`：通用能力与静态资源

这样做的目的是让业务代码靠近所属模块，同时避免把所有通用代码堆成一个失控的大目录。

## 桌面后端

Rust 代码按职责组织：

- `commands`：Tauri 命令入口
- `application`：应用启动与共享状态
- `domain`：领域模型与通用数据结构
- `repositories`：SQLite 读写
- `services`：下载队列、Python 运行桥接等服务层能力

## Python 运行时

Python 代码拆分为：

- `core`：协议入口、请求分发与运行流程
- `tasks`：具体任务处理实现

`main.py` 只保留轻量入口职责，避免把任务逻辑堆进单文件。
