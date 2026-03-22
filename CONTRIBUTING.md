# 贡献指南

感谢你关注 `Dr.Tools`。

本项目仍处于开发阶段，功能、结构和实现会持续演进。为了保证项目质量、协作效率和开源规范，所有贡献者都需要遵守以下约束。

## 项目原则

- 本项目是开源项目，所有贡献都必须遵循开源精神。
- 项目中的视频下载与直播录制能力基于开源项目 `F2` 集成与扩展，贡献时必须尊重原项目及相关依赖的来源与边界。
- 不允许提交会破坏现有功能、破坏结构边界、或明显降低可维护性的改动。
- 在没有明确需求的前提下，不要擅自改变既有交互、布局、命名或行为。

## 开发环境要求

- Node.js `20+`
- `pnpm`
- Rust 工具链
- Python `3.12`
- Tauri 2 开发环境

## 强制约束

### 包管理与命令

- 只能使用 `pnpm`
- 不要使用 `npm`
- 不要提交与 `pnpm-lock.yaml` 不一致的依赖变更

### 项目结构

- 前端业务代码必须放在 `src/modules`
- 通用前端能力放在 `src/api`、`src/assets`、`src/i18n`、`src/lib`、`src/stores`、`src/theme`
- 不要把无关逻辑塞进单个页面或单个目录
- 新增功能时优先延续现有模块边界，不要随意发散结构

### Tauri / Rust / Python 职责边界

- Tauri command 层只负责参数校验、入口协调和返回结果
- Rust 业务逻辑优先下沉到 `services` 或 `repositories`
- 数据结构统一放在 `src-tauri/src/domain`
- Python `main.py` 只负责协议入口
- Python 任务分发统一放在 `src-tauri/python/core`
- 具体任务实现放在 `src-tauri/python/tasks`

### UI 与交互

- 所有界面改动都必须同时兼容浅色模式和深色模式
- 不要引入与现有设计语言冲突的组件风格
- 不要随意增加说明性文案、标题、副标题或装饰性布局
- 滚动区域默认隐藏滚动条，除非需求明确要求显示

### 文档与版本控制

- `README.md`、`CONTRIBUTING.md`、`LICENSE` 这类根目录文档需要维护
- `docs/` 目录当前不纳入版本控制，不要提交 `docs/` 下的内容
- 文档更新必须与当前代码状态一致，不要写与实现不符的说明

## 代码规范

- 保持文件职责单一
- 命名清晰、稳定、可读
- 避免重复逻辑
- 注释保持克制，只在必要时解释非显而易见的内容
- 不要为了小改动做大范围重构
- 不要混入无关清理、无关格式化、无关重命名

## 提交规范

提交信息必须使用约定式前缀，格式如下：

```text
<type>: <summary>
```

允许使用的 `type`：

- `fix`: 修复 bug 或已知问题
- `feat`: 添加新特性
- `docs`: 只涉及文档更新
- `style`: 代码格式、空格、逗号等（不影响代码功能的变动）
- `refactor`: 代码重构（不增加新特性或修复 bug）
- `test`: 添加测试
- `chore`: 其他更改（如构建过程、依赖项更新等）

示例：

```text
feat: add live recording scheduler
fix: handle recording task interruption
docs: update README project status
chore: ignore docs directory
```

## 提交前检查

提交前至少执行：

```bash
pnpm typecheck
pnpm build
pnpm check:desktop
```

如果改动涉及 Python：

```bash
python3 -m py_compile src-tauri/python/**/*.py
```

如果改动涉及 Rust：

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

如果你没有运行某项检查，不要假设它一定没问题。

## 禁止事项

- 不要使用 `npm`
- 不要随意提交 `docs/`
- 不要提交与当前需求无关的大范围改动
- 不要擅自删除现有功能
- 不要绕开现有模块边界硬塞实现
- 不要把临时调试代码、日志输出、测试数据直接带入正式提交
- 不要使用破坏性 Git 命令覆盖他人改动

## Pull Request 自查清单

提交前请确认：

- 改动与需求直接相关
- 没有破坏已有功能
- 浅色 / 深色模式都已检查
- 使用了正确的提交前缀
- 已执行必要校验命令
- 没有误提交 `docs/`、临时文件、日志文件或调试代码

## 额外说明

- 如果你准备做结构调整，请先证明它服务于当前需求，而不是出于个人偏好
- 如果你准备引入新依赖，请先确认现有依赖无法解决问题
- 如果你准备修改公共能力，请先评估是否会影响现有模块
