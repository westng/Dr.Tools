# 贡献指南

## 开发规则

- 使用 `pnpm`，不要使用 `npm`
- 除非需求明确要求，否则不要随意改变现有功能行为
- 业务代码放在对应模块目录 `src/modules`
- 通用代码放在 `src/api`、`src/i18n`、`src/lib`、`src/stores`、`src/theme`、`src/assets`
- Tauri command 文件保持轻量，业务逻辑优先下沉到 Rust 的 services 或 repositories
- Python `main.py` 保持轻量，任务分发放在 `src-tauri/python/core`

## 提交前检查

请至少执行以下命令：

```bash
pnpm build
pnpm check:desktop
```

如果改动涉及 Python 任务处理逻辑，请额外执行本地 Python 语法检查。

## 代码风格

- 命名清晰，文件职责单一
- 避免重复业务逻辑
- 不要无理由做大范围结构改动
- 注释保持克制，只写必要内容
