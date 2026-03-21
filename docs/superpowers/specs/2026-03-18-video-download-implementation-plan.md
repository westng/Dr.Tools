# 视频下载实现计划

日期：2026-03-18  
依赖：`docs/superpowers/specs/2026-03-18-video-download-design.md`  
范围：`video.download` 的第一轮实现

## 1. 目标

实现首个可用版的视频下载能力，约束如下：

- 平台：`douyin`、`tiktok`
- 输入：一条或多条直链视频链接
- 批量模式：一行一条链接
- 执行模型：一条链接一个任务
- 队列模型：设置中统一管理全局并发上限
- 下载内容：视频文件 + 基础元数据，附加内容可选

该计划只覆盖第一条稳定可用链路，不提前实现延后功能。

## 2. 交付策略

按纵向切片交付，不按层横切拆散实现。

建议顺序：

1. 数据契约与设置字段
2. 前端页面状态与校验
3. Rust 命令与任务创建
4. 队列执行器与并发控制
5. Python `f2` 桥接
6. 任务记录与结果反馈
7. 验证与收口

这样可以缩短关键路径，避免先做完 UI 却没有可执行后端。

## 3. Slice 1：共享契约与设置

### 3.1 前端类型

增加以下类型：

- `DownloadPlatform = 'douyin' | 'tiktok'`
- `VideoDownloadSubmitPayload`
- `VideoDownloadSubmitResult`

前端载荷示例：

```ts
interface VideoDownloadSubmitPayload {
  platform: 'douyin' | 'tiktok';
  urls: string[];
  downloadCover: boolean;
}
```

前端返回示例：

```ts
interface VideoDownloadSubmitResult {
  createdTaskIds: string[];
  acceptedCount: number;
  skippedCount: number;
  invalidUrls: string[];
}
```

### 3.2 设置模型

新增一个全局设置项：

- `maxConcurrentDownloads: number`

涉及位置：

- `src/modules/settings/types.ts`
- `src/modules/settings/stores/settings.store.ts`
- `src/modules/settings/api/settings.api.ts`
- Rust 设置模型与持久化
- 设置校验路径

### 3.3 默认值与校验

初始建议：

- 默认值：`3`
- 允许范围：`1..=8`

校验必须在 Rust 侧存在，不能只依赖前端。

## 4. Slice 2：下载页 UI

### 4.1 替换占位页

将 `src/modules/download/pages/VideoDownloadPage.vue` 从占位页替换为首个可用页面。

规划区块：

- 输入区
- 参数区
- 执行区
- 处理日志区

### 4.2 页面本地状态

本地状态至少包含：

- `platform`
- `rawInput`
- `additionalOptions`
- `submitting`
- `validationError`
- `currentBatchId`
- `batchLogs`

### 4.3 输入标准化

前端提交前执行：

- 按换行分割
- 裁剪首尾空白
- 丢弃空行
- 精确去重

v1 不做完全 URL 规范化。

### 4.4 UI 行为

要求：

- 标准化后为空时禁止提交
- 全部为空时给出明确校验提示
- 命令返回后刷新日志区或批次关联数据
- 提供进入任务记录 / 批次明细的入口

### 4.5 本切片不包含

不增加：

- 解析预览
- 每条链接可编辑行
- 重试控制
- 高级参数面板

## 5. Slice 3：Rust 命令面

### 5.1 新增 Tauri 命令

新增独立提交命令，而不是复用旧的任务占位 API。

建议命令：

- `video_download_submit`

该命令需要：

1. 校验载荷
2. 防御式标准化或复核链接
3. 为每条有效链接创建任务记录
4. 把任务加入队列
5. 返回提交结果摘要

### 5.2 Rust 载荷类型

在 Rust 中定义清晰的请求 / 响应结构：

- 提交载荷
- 提交结果

校验规则：

- 平台必须受支持
- 标准化后链接不能为空
- 单次提交数量应有上限保护

初始建议：

- 单次最多 `100` 条链接

### 5.3 命令输出

返回前端需要的数据：

- 创建任务数量
- 跳过 / 无效数量
- 创建的任务 ID 列表
- 无效链接列表

## 6. Slice 4：SQLite 与任务记录

### 6.1 复用现有任务表

v1 默认不新增第二张下载专属任务表，除非现有结构确实无法支撑。

优先扩展现有任务载荷 / 结果约定。

每个创建的任务至少包含：

- `task_type = video.download`
- 序列化任务载荷：
  - `platform`
  - `sourceUrl`
  - `additionalOptions`

### 6.2 可选结构扩展

只有当现有表无法支持筛选或展示时，才增加列。

如果必须扩展，优先最小化，例如：

- `platform`
- `source_url`
- `batch_id`

默认建议仍是首版避免无谓 schema 震荡。

### 6.3 任务状态流转

目标状态序列：

- `queued`
- `running`
- `success`
- `failed`

如果当前 schema 用词略有不同，统一映射即可，不要并行发明另一套状态。

## 7. Slice 5：队列执行器与并发

### 7.1 队列归属

Rust 负责调度与并发上限控制。

原因：

- 设置本身就在 Rust 侧
- 队列状态需要一个单一可信源
- Python 应只关注执行，不负责调度策略

### 7.2 并发规则

队列执行器行为：

- 读取 `maxConcurrentDownloads`
- 同时最多运行 N 个 `video.download` 任务
- 其他任务保持排队
- 某个任务完成后，立即拉起下一个排队任务

### 7.3 v1 范围

不实现：

- 暂停 / 恢复
- 手动排序
- 按平台分桶并发
- 多任务类型公平调度

v1 只做有上限的简单 FIFO 队列。

## 8. Slice 6：Python 与 `f2` 桥接

### 8.1 桥接设计

在 Python sidecar 中为 `video.download` 增加独立执行路径。

高层流程：

1. Rust 发送任务载荷
2. Python 按 `taskType` 分发
3. `video.download` 处理器把平台与选项映射到 `f2`
4. Python 返回结构化成功 / 失败结果

### 8.2 包装边界

v1 不要把 `f2` 调用散落到很多文件里。

优先保留一个轻量包装模块，提供清晰入口：

- 解析载荷
- 调用 `f2`
- 捕获输出路径、元数据路径、封面 / 原声 / 文案 / 歌词结果
- 返回结构化结果

### 8.3 Python 结果结构

结果至少包含：

- `sourceUrl`
- `platform`
- `outputPath`
- `metadataPath`
- `coverPath`（如存在）
- `audioPath`（如存在）
- `captionPath`（如存在）
- `lyricPath`（如存在）
- `message`

### 8.4 失败处理

Python 应尽量返回结构化失败信息，而不是只透出原始 stderr 字符串。

## 9. Slice 7：设置页改动

在 `src/modules/settings/pages/SettingsPage.vue` 中增加下载设置区块。

初始内容：

- `最大并发下载数`

行为：

- 复用当前设置加载 / 保存流程
- 输入时校验
- 通过现有设置更新命令持久化

该项必须保持为全局系统设置，而不是单次任务选项。

## 10. Slice 8：任务记录与反馈

### 10.1 任务记录页

最低可用更新：

- 能清晰区分 `video.download` 任务
- 尽可能展示平台信息
- 展示来源链接或可识别的短标识
- 按批次查看子任务

### 10.2 提交反馈

下载页提交后应立即展示：

- 创建任务数量
- 无效 / 跳过数量
- 到 `任务记录` 的明确入口

### 10.3 延后内容

v1 不做完整进度仪表盘。

## 11. 文件级工作拆分

### 前端

- `src/modules/download/pages/VideoDownloadPage.vue`
- `src/modules/download/api/download.api.ts`
- `src/modules/download/types.ts`
- `src/modules/settings/pages/SettingsPage.vue`
- `src/modules/settings/types.ts`
- `src/modules/settings/stores/settings.store.ts`
- `src/modules/settings/api/settings.api.ts`
- `src/modules/tasks/pages/HistoryPage.vue`
- `src/modules/tasks/pages/BatchDetailPage.vue`
- `src/modules/tasks/types.ts`
- `src/modules/tasks/api/tasks.api.ts`

### Rust

- `src-tauri/src/commands/...`
- `src-tauri/src/repositories/...`
- `src-tauri/src/services/...`
- 设置持久化与校验相关模块

### Python

- `src-tauri/python/core/...`
- `src-tauri/python/tasks/...`
- `src-tauri/python/main.py`
- 新增 `f2` 包装模块

## 12. 验证计划

### 前端

- 混合空行输入的标准化行为
- 重复链接去重
- 空提交拦截
- 提交后日志与反馈展示

### Rust

- 载荷校验
- 一条链接对应一个任务创建
- 队列入队行为
- 并发上限生效

### Python

- `video.download` 任务分发
- 成功结果结构
- 结构化失败路径

### 端到端

- 提交一条有效链接
- 提交多条有效链接
- 提交有效与无效混合链接
- 将并发设为 `1`，验证串行执行

## 13. 风险与缓解

### 风险 1：`f2` 集成复杂度高于预期

缓解：

- 保持 Rust / Python 契约最小化
- 把 `f2` 细节隔离在单一 Python 包装层中

### 风险 2：现有任务表过于通用

缓解：

- 先复用当前表
- 只有当任务记录展示或筛选明显受限时，再补充 schema

### 风险 3：队列执行器被过度设计

缓解：

- v1 只实现有上限的 FIFO 队列

## 14. 建议的实现顺序

严格按以下顺序执行：

1. 增加设置字段 `maxConcurrentDownloads`
2. 增加前端下载提交 API / 类型契约
3. 构建 `VideoDownloadPage` UI 与输入标准化
4. 增加 Rust `video_download_submit` 命令
5. 为每条有效 URL 写入一条 SQLite 任务
6. 实现 Rust 队列执行器与全局并发上限
7. 增加 Python `video.download` 处理器并接入 `f2`
8. 改进任务记录页对下载任务的展示
9. 进行端到端验证
