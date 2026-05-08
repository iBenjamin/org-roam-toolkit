---
description: 添加工具/资源到季度 toolkit 收藏
argument-hint: <name> <url>
---

用户要把一个工具/库/服务/API 加入 toolkit 收藏（季度文件 `toolkit/YYYY-QN.org`，**不是** org-roam 节点）。

输入: $ARGUMENTS

## 步骤

1. 解析参数：第一个 token 是 `name`，第二个是 `url`，剩余是描述
2. 如果有 URL，用 `WebFetch` 验证可达 + 抓取一句话描述（如果用户没自带描述）
3. 调用 `roam_add_toolkit` MCP tool：
   - `title`: 资源名称
   - `url`: 资源 URL
   - `tags`: 分类标签（参考下表）
   - `category`: `"library"` / `"tool"` / `"service"` / `"api"`
   - `description`: 一句话描述
4. 确认添加成功，返回季度文件中的位置

## 标签体系

| 维度 | 标签示例 |
|------|----------|
| 类型 | `library`, `tool`, `service`, `api` |
| 领域 | `ios`, `macos`, `web`, `design`, `marketing`, `ai` |
| 特殊 | `free`, `paid`, `starred` |

## 示例

| 输入 | 解析 |
|---|---|
| `SnapKit https://github.com/SnapKit/SnapKit iOS Auto Layout DSL` | name=SnapKit, url=..., description=iOS Auto Layout DSL, category=library, tags=[library, ios, ui] |
| `Raycast https://www.raycast.com/` | name=Raycast, url=..., 抓取得到 description, category=tool, tags=[tool, macos, productivity] |
| `OpenAI API https://platform.openai.com/` | category=api, tags=[api, ai, llm] |
