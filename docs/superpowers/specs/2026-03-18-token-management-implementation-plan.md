# Token 管理实现计划

日期：2026-03-18  
依赖：`docs/superpowers/specs/2026-03-18-token-management-design.md`  
范围：平台 Cookie 管理的第一轮实现

## 1. 目标

交付一个可用的 Token 管理 v1，包含：

- 按平台存储 Cookie
- 状态跟踪
- 手动校验
- 下载页提示
- 不做浏览器自动化

## 2. 交付切片

### Slice 1：共享类型与设置字段

为两个平台增加设置字段：

- cookie
- updatedAt
- lastCheckedAt
- lastCheckStatus
- lastCheckMessage

涉及位置：

- 前端设置类型
- 设置 store
- Rust 设置模型
- Rust 持久化键值

### Slice 2：校验命令面

新增独立的 Tauri 命令：

- `token_validate`

该命令需要：

1. 校验平台
2. 标准化 Cookie
3. 调用隔离的 Python 校验逻辑
4. 持久化返回结果
5. 返回类型明确的响应给前端

### Slice 3：Python 校验任务

在 Python 中增加 `token.validate` 任务处理。

平台策略：

- `douyin`：走 `f2` 请求路径
- `tiktok`：走启发式校验路径

如果只是瞬时失败，不应直接写成硬性无效状态。

### Slice 4：设置页 UI

将 `SettingsPage.vue` 按区块整理为：

- 外观
- 主题
- 下载
- Token 管理
- 更新

增加总览卡片和两个平台卡片，支持保存 / 清空 / 校验 / 指引动作。

### Slice 5：下载页提示

更新 `VideoDownloadPage.vue`，基于当前所选平台展示 Token 风险提示。

规则：

- 未配置时提示
- `unchecked` 时提示
- `invalid` / `expired` 显示危险态
- v1 不阻止提交

### Slice 6：验证

执行：

- `pnpm build`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `.venv/bin/python -m py_compile src-tauri/python/main.py src-tauri/python/tasks/video_download.py src-tauri/python/tasks/token_validate.py`

## 3. 延后事项

后续可继续补：

- 浏览器 Cookie 导入
- 更清晰的一键获取路径
- 从下载页警告直接跳转设置
- 更强的平台专属校验
- 多账号支持
