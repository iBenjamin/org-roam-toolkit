---
description: 深度学习一篇文章：保存 reference + 创建原子笔记 + 织入知识图谱
argument-hint: <url>
---

用户要深度学习这篇文章。需要完成两件事：
1. 将文章蒸馏为 reference 笔记
2. 从文章中提炼原子概念，创建独立笔记，织入知识图谱

URL: $ARGUMENTS

**所有格式 / References / AI 标记 / 织入图谱规范**：见 `atomic-notes` skill。

## 第一步：获取文章内容

使用 `fetch` skill 获取文章全文（微信文章会自动走 wechat 站点策略）。

## 第二步：创建 Reference 笔记

对文章进行知识蒸馏：
- 提炼核心概念和关键洞察
- 用 org-mode 格式重构（标题层级、代码块、表格等）
- 保留关键代码示例和数据
- 去除废话和冗余，只留干货

调用 `roam_create_note`：
- `title`: 文章原标题（禁止 AI 改写或精炼，保持原文标题）
- `subdirectory`: `"reference"`
- `sourceUrl`: 原始 URL
- `tags`: 包含分类标签 + AI 生成标签（按 `atomic-notes` skill 规范）
- `properties`: 按 `atomic-notes` skill "AI 生成内容标记" 小节
- `content`: 蒸馏后的 org-mode 内容（reference 笔记不强制 :ZH: drawer 双语化，保留原文语言）

## 第三步：识别原子概念

从文章内容中识别值得独立成笔记的核心概念：
- 每个概念必须是独立的知识单元，脱离原文仍有意义
- 宁少勿多，只提炼真正有价值的概念（通常 2-5 个）
- 概念粒度：一个概念 = 一句话能说清本质 + 几个要点展开

## 第四步：查重 & 创建原子笔记

对每个识别出的概念：

1. `roam_search_title` 查重
2. 已存在：跳过创建，但记录下来用于后续链接
3. 不存在：调用 `roam_create_note`：
   - `subdirectory`: `"main"`
   - `tags`: 分类标签 + AI 生成标签
   - 全部按 `atomic-notes` skill 规范：双语标题、:ZH: drawer、双层 References、AI 标记

## 第五步：织入知识图谱

按 `atomic-notes` skill 的"织入图谱原则"：
1. 每个原子笔记 ↔ reference 笔记（双向）
2. 相关原子笔记之间互相链接
3. `roam_search_title` / `roam_search_tag` 找已有相关笔记建立链接

## 第六步：提交 archive.today

收集所有 References URL，按 `atomic-notes` skill 的"提交 archive.today"小节执行。

## 第七步：输出摘要

- Reference 笔记路径
- 新建原子笔记列表（标题 + 路径）
- 建立的链接关系
- 跳过的已有笔记（如有）
