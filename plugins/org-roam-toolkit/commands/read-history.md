---
description: 将已读文章加入阅读历史
argument-hint: <url>
---

用户已经阅读了这篇文章，加入阅读历史（季度文件 `read_history/YYYY-QN.org`，**不是** org-roam 节点）。

URL: $ARGUMENTS

## 步骤

1. 使用 `fetch` skill 获取文章内容（微信文章自动走 wechat 站点策略）
2. 调用 `roam_add_reading_history` MCP tool：
   - `title`: 文章标题
   - `url`: `$ARGUMENTS`
   - `tags`: 分类标签
   - `source`: 网站名（`cnblogs`、`github`、`weixin` 等）
   - `author`: 作者名（如果抓得到）
   - `summary`: 一句话摘要（如果文章为空，例如整篇是图片，**不传** `summary`）
   - `points`: 关键要点列表（2-5 条）
   - `rating`: 1-5 评分（可选）
3. 确认添加成功，返回季度文件中的位置
