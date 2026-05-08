---
description: 分析 reference 笔记，提取原子概念及其 1 级相关概念
argument-hint: <reference 笔记标题关键词>
---

用户要从一篇已有的 reference 笔记中深度提取知识，创建原子笔记网络。

关键词：$ARGUMENTS

**所有格式 / References / AI 标记 / 织入图谱规范**：见 `atomic-notes` skill。

## 第一步：定位 Reference 笔记

调用 `roam_search_title` 搜索 reference 笔记：
- 唯一匹配：继续
- 多个匹配：列出让用户选择
- 找不到：提示用户检查标题

## 第二步：读取 Reference 内容

通过 emacsclient 读取笔记全文：

```bash
emacsclient -e '(with-temp-buffer (insert-file-contents "<file-path>") (buffer-string))'
```

## 第三步：识别原子概念

从 reference 内容中识别值得独立成笔记的核心概念：
- 每个概念必须是独立的知识单元，脱离原文仍有意义
- 宁少勿多，2-5 个真正有价值的概念
- 概念粒度：一句话说本质 + 几个要点展开
- 跳过过于宽泛的概念（"架构"、"设计模式"）
- 跳过已有笔记（`roam_search_title` 批量查重）

## 第四步：展开 1 级相关概念

对每个原子概念，按 `atomic-notes` skill 的"织入图谱原则"识别其 1 级相关概念（每个 2-5 个），批量 `roam_search_title` 查重。

## 第五步：创建原子笔记

对每个需要新建的原子概念，`roam_create_note`：
- `subdirectory: "main"`、`tags`、AI 标记 properties：按 `atomic-notes` skill
- `content`：按 `atomic-notes` skill 规范，**额外包含**：
  - 一句话本质定义
  - 核心要点展开
  - "相关概念"段落：1 级相关概念用 `[[id:xxx]]` 链接
  - 关键代码/数据示例
  - References（WebSearch 权威来源，至少 2-3 条）

## 第六步：创建 1 级相关概念笔记

对需要新建的 1 级相关概念，`roam_create_note`：
- 同上格式规范
- 但**不递归展开**其相关概念段落

## 第七步：织入知识图谱

按 `atomic-notes` skill 的"织入图谱原则"：
1. 每个原子笔记 ↔ 源 reference 笔记（双向）
2. 原子笔记 ↔ 其 1 级相关概念（双向）
3. 原子笔记之间有逻辑关联的互相链接
4. `roam_search_title` / `roam_search_tag` 找已有笔记建立链接

## 第八步：提交 archive.today

按 `atomic-notes` skill 的"提交 archive.today"小节，逐条 `browse-url`，间隔 1 秒。

## 第九步：输出摘要

- 源 Reference 笔记标题和路径
- 提取的原子概念列表
- 新建的原子笔记（标题 + 路径）
- 新建的 1 级相关概念笔记列表
- 跳过的已有笔记
- 建立的链接关系图
