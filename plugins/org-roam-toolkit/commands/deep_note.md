---
description: 对一个概念创建原子笔记，并对其 1 级相关概念也创建原子笔记
argument-hint: <概念名称>
---

用户要为一个概念及其直接相关概念创建原子笔记网络。

概念：$ARGUMENTS

**所有格式 / References / AI 标记 / 织入图谱规范**：见 `atomic-notes` skill。1 级相关概念的判定标准也在那里（"织入图谱原则"小节）。

## 第一步：理解核心概念 & 识别 1 级相关

基于你对该概念的深度理解：
1. 核心概念本身的本质定义
2. 该概念的 1 级相关概念（按 `atomic-notes` skill 判定标准，3-7 个）

## 第二步：批量查重

对核心概念和所有 1 级相关概念，调用 `roam_search_title` 检查是否已存在。
- 已存在：记录用于后续建立链接（不重复创建）
- 不存在：记录待新建

## 第三步：创建核心概念笔记

调用 `roam_create_note`：
- `title`、`subdirectory: "main"`、`tags`、AI 标记 properties：按 `atomic-notes` skill
- `content`：按 `atomic-notes` skill 规范，**额外包含**：
  - 一句话本质定义
  - 核心要点展开
  - "相关概念"段落：列出所有 1 级相关概念，使用 `[[id:xxx]]` 链接格式
  - 关键代码/数据示例（如有）
  - References（用 WebSearch 搜索权威来源，至少 2-3 条）

## 第四步：创建 1 级相关概念笔记

对每个需要新建的 1 级相关概念，调用 `roam_create_note`：
- 同核心概念笔记的格式规范
- 但**不再递归展开**其相关概念段落（避免无限递归）

## 第五步：织入知识图谱

按 `atomic-notes` skill 的"织入图谱原则"：
1. 核心概念 ↔ 每个 1 级相关概念（双向）
2. 1 级相关概念之间有逻辑关联的互相链接
3. `roam_search_title` / `roam_search_tag` 找已有笔记建立链接

## 第六步：提交 archive.today

按 `atomic-notes` skill 的"提交 archive.today"小节，收集所有新建笔记 References URL 逐条 `browse-url`，间隔 1 秒。

## 第七步：输出摘要

- 核心概念笔记路径
- 新建的 1 级相关概念笔记列表
- 跳过的已有笔记
- 建立的链接关系图
