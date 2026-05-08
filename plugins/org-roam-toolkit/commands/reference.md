---
description: 将有价值的文章放到 org-roam reference 中
argument-hint: <url or local-file-path>
---

用户认为这篇文章有价值，需要存入 org-roam reference 笔记。

输入: $ARGUMENTS

**AI 标记规范**：见 `atomic-notes` skill 的"AI 生成内容标记"小节。Reference 笔记**始终**标记 AI 生成。

判断输入类型：
- `http://` 或 `https://` 开头 → URL 流程
- 否则 → 本地文件流程

---

## URL 流程

1. 使用 `fetch` skill 获取文章内容（微信文章自动走 wechat 站点策略）
2. 提取标题，对文章进行知识蒸馏：
   - 提炼核心概念和关键洞察
   - 用 org-mode 格式重构（标题层级、代码块、表格等）
   - 保留关键代码示例和数据
   - 去除废话和冗余，只留干货
3. 确定分类标签
4. 调用 `roam_create_note`：
   - `title`: 文章原标题（禁止 AI 改写或精炼）
   - `subdirectory`: `"reference"`
   - `sourceUrl`: 原始 URL
   - `tags`: 分类标签 + AI 生成标签
   - `properties`: 按 `atomic-notes` skill 的 AI 标记规范
   - `content`: 蒸馏后的 org-mode 内容
5. 返回笔记路径

---

## 本地文件流程

本地文件没有稳定外部来源，reference 笔记就是唯一持久化副本，**必须全量保留，不蒸馏不删减**。

1. 用 `Read` 工具读取本地文件内容
2. 提取标题（取文件中的一级标题，没有则用文件名）
3. 全量转换为 org-mode 格式（markdown → org，不删减任何内容）
4. 确定分类标签
5. 调用 `roam_create_note`：
   - `title`: 文章原标题
   - `subdirectory`: `"reference"`
   - `sourceUrl`: 不设置（无外部来源）
   - `tags`: 分类标签 + AI 生成标签
   - `properties`: 按 `atomic-notes` skill 的 AI 标记规范，**额外加** `LOCAL_SOURCE: <原始文件路径>`
   - `content`: 全量 org-mode 内容
6. 返回笔记路径
