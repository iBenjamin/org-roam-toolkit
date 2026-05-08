---
description: 将 URL 加入待读列表
argument-hint: <url>
---

用户想稍后阅读这篇文章。

URL: $ARGUMENTS

## 步骤

1. 用 `WebFetch` 抓取页面，提取标题，生成一句话说明（≤ 15 字）
2. 调用 `roam_add_to_read` MCP tool：
   - `title`: 文章标题
   - `url`: `$ARGUMENTS`
   - `summary`: 一句话说明（如果抓不到内容，可省略 summary）
3. 确认添加成功，返回 todo.org Inbox 中的位置
