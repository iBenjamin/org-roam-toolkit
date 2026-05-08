---
description: 创建一个概念的原子笔记，织入知识图谱
argument-hint: <概念名称>
---

用户要为一个概念创建原子笔记。

概念：$ARGUMENTS

**所有格式 / References / AI 标记 / 织入图谱规范**：见 `atomic-notes` skill。本 command 只描述工作流。

## 第一步：查重

调用 `roam_search_title` 检查是否已存在同名或近似笔记。
- 已存在：告知用户，询问是否补充/更新（不自动覆盖）
- 不存在：继续

## 第二步：创建原子笔记

调用 `roam_create_note`：
- `title`: 按 `atomic-notes` skill 的双语标题规范
- `subdirectory`: `"main"`
- `tags`: 分类标签
- `content`: 按 `atomic-notes` skill 的双语内容规范，包含 References 双层结构
- 标记 AI 生成（按 `atomic-notes` skill 的 Properties + FILETAGS 规范）

References 必须真实——用 `WebSearch` 搜索该概念的权威来源，至少 2-3 条。

## 第三步：织入知识图谱

按 `atomic-notes` skill 的"织入图谱原则"：
1. `roam_search_title` / `roam_search_tag` 找已有相关笔记
2. `roam_create_link` 双向链接

## 第四步：提交 archive.today

按 `atomic-notes` skill 的"提交 archive.today"小节，收集 References 中所有原始 URL，逐条 `emacsclient -e '(browse-url "...")'`，每条间隔 1 秒。

## 第五步：输出摘要

- 笔记路径
- 建立的链接关系（数量 + 关键节点名）
