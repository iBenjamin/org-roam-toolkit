---
name: atomic-notes
description: |
  原子笔记 (org-roam) 创建与管理规范：双语标题/标签、:ZH: drawer 折叠中文、
  双层 References (inline + 汇总 + archive.today)、AI 生成内容标记、织入知识图谱原则。
  当 commands 创建笔记或对话中讨论笔记格式时自动应用。

  Triggers: 原子笔记, atomic note, 概念笔记, Zettelkasten, roam note,
  org-roam, 双语笔记, :ZH:, drawer, FILETAGS, 织入图谱, References,
  archive.today, ai_generated, 双向链接
---

# Atomic Notes Skill

org-roam 原子笔记的格式规范、引用规范、图谱织入原则。

**适用范围**：所有创建或修改 org-roam 节点笔记的工作。Commands `/note`、`/study`、`/deep_note`、`/reference`、`/ref-extract` 都按这套规范工作；自由对话中创建笔记也应遵循。

---

## 1. 双语标题规范

格式：`English Name 中文名称 (ABBR)`

规则：
- 英文名在前，中文名在后，空格分隔
- 如果有常用缩略词，放在末尾括号内
- 如果概念没有对应中文名（人名、专有产品名），保留原文即可

示例：
- `Knowledge Distillation 知识蒸馏 (KD)`
- `Backpropagation 反向传播 (BP)`
- `Model Watermarking 模型水印`
- `Attention Mechanism 注意力机制`
- `Remote Procedure Call 远程过程调用 (RPC)`

## 2. 双语标签规范

`#+FILETAGS` 必须同时包含英文和中文标签。

格式：`#+FILETAGS: :English_Tag:中文标签:English_Tag:中文标签:`

规则：
- 每个标签概念同时提供英文版和中文版
- 英文标签用下划线连接多词（org-mode 标签不支持空格）
- 如果有常用缩略词，全称和缩略词都要保留：`:Intellectual_Property:IP:知识产权:`
- 英文标签紧跟对应的中文标签，成对出现

示例：
- `#+FILETAGS: :AI_Security:AI安全:Model_Attack:模型攻击:`
- `#+FILETAGS: :Machine_Learning:机器学习:Model_Compression:模型压缩:`
- `#+FILETAGS: :Cryptography:密码学:Authentication:认证:`

### 调用 `roam_create_note` 时

`tags` 数组必须遵循同样的"成对"规则：

```jsonc
// ✅ 正确：每个概念双语成对
"tags": ["AI_Security", "AI安全", "Model_Attack", "模型攻击"]

// ❌ 错误：单语
"tags": ["ai-security", "model-attack"]
```

## 3. 双语内容规范

**核心原则**：英文做主线阅读流，中文用 `:ZH:...:END:` drawer 折叠（Emacs 中 TAB 展开）。

### 结构

```org
English one-line definition.

:ZH:
中文一句话本质定义。
:END:

** English Title 中文标题

English paragraph content.

:ZH:
中文段落内容。
:END:

** Another Title 另一个标题

- *English Term*: English description
- *English Term*: English description

:ZH:
- *中文术语*: 中文描述
- *中文术语*: 中文描述
:END:
```

### 规则

1. **开篇定义**：英文一句话定义，紧跟 `:ZH:` drawer 放中文定义
2. **二级标题**：双语并列 `** English Title 中文标题`（英文在前）
3. **段落**：英文写完整段落，段落后用 `:ZH:...:END:` drawer 放中文翻译
4. **列表项**：英文列表写完，整个列表后用一个 `:ZH:...:END:` drawer 放中文版列表
5. **代码块**：不翻译，不放进 drawer
6. **References**：不翻译，不放进 drawer

## 4. References 双层规范

笔记中的外部引用走两层结构：

### 第一层：inline 引用

正文中提到具体来源时，直接在旁边附 org-mode 链接：

```org
Google 2024 年多次更新核心算法专门打击 AI 生成的垃圾内容
（[[https://blog.google/...][Google Blog]]）。
```

### 第二层：References 汇总段落

笔记末尾添加 `* References` 段落，汇总正文中所有外部链接，每条附 archive.today 提交链接：

```org
* References

- [[https://blog.google/...][Google Blog: Fighting spam]] | [[https://archive.today/submit/?url=https%3A%2F%2Fblog.google%2F...][archive]]
- [[https://example.com][Source Title]] | [[https://archive.today/submit/?url=https%3A%2F%2Fexample.com][archive]]
```

要求：
- archive.today 链接格式：`https://archive.today/submit/?url=` + URL 编码后的原始链接
- 至少提供 2-3 条外部引用（除非概念极其小众）
- 正文 inline 引用和末尾 References 的链接保持一致
- 真实 URL —— 用 WebSearch 搜索权威来源，不能编造

### 提交 archive.today

创建笔记后，收集所有 inline / References 中的原始 URL（非 archive.today 链接），通过 emacsclient 调用 `browse-url` 逐一打开 archive.today 提交页面：

```bash
emacsclient -e '(browse-url "https://archive.today/submit/?url=<URL 编码后的原始链接>")'
```

每条 URL 间隔 1 秒（`sleep 1`），避免浏览器卡顿。

## 5. AI 生成内容标记

所有 AI 生成的笔记 MUST 清晰标记，方便识别潜在幻觉。

### 标记规范

1. **Tag**：FILETAGS 中加入 `:AI_Generated:AI生成:`
2. **Properties drawer**：
   - `GENERATOR`: AI 系统名（例 `claude`）
   - `MODEL`: 模型版本（例 `opus-4.7`）
   - `GENERATED_AT`: 生成时间戳（例 `[2026-05-08 Wed]`）

### 完整示例

```org
#+FILETAGS: :AI_Generated:AI生成:Cryptography:密码学:
:PROPERTIES:
:ID:           xxxx-xxxx
:GENERATOR:    claude
:MODEL:        opus-4.7
:GENERATED_AT: [2026-05-08 Wed]
:END:
#+title: Knowledge Distillation 知识蒸馏 (KD)

A model compression technique where a smaller "student" model learns to mimic a larger "teacher" model.

:ZH:
通过让"学生"小模型学习"教师"大模型行为来压缩模型的技术。
:END:
```

### 适用范围

- `/note`、`/deep_note`、`/ref-extract` 创建的原子笔记：标记 AI 生成
- `/study`、`/reference` 创建的 reference 笔记：**始终**标记 AI 生成
- 用户手动通过 emacsclient 直接编辑的笔记：不标记

## 6. 织入知识图谱原则

每个新建笔记必须主动找已有相关笔记建立链接。

### 工作流

1. **查相关**：用 `roam_search_title` / `roam_search_tag` 找已有相关节点
2. **建双向链接**：用 `roam_create_link` (with `bidirectional: true`) 建 source ↔ target
3. **跨笔记关联**：批量创建多个笔记时，相关概念之间也互相链接

### 1 级相关概念判定标准（用于 `/deep_note`、`/ref-extract`）

- 必须是理解核心概念所必需的前置知识
- 或者是核心概念的核心组成部分
- 或者是与核心概念形成重要对比的概念
- 数量控制在 3-7 个，宁少勿多

### 跳过条件

- 已存在同名/近似笔记 → **跳过创建**，但**仍然建立链接**
- 概念过于宽泛（"架构"、"设计模式"）→ 跳过创建，不勉强建节点

## 7. 完整示例（含全部规范）

```org
:PROPERTIES:
:ID:           a1b2c3d4-e5f6-7890-abcd-ef1234567890
:GENERATOR:    claude
:MODEL:        opus-4.7
:GENERATED_AT: [2026-05-08 Wed]
:END:
#+FILETAGS: :AI_Generated:AI生成:Machine_Learning:机器学习:Model_Compression:模型压缩:
#+title: Knowledge Distillation 知识蒸馏 (KD)

A model compression technique where a smaller "student" model learns to mimic the soft probability distributions of a larger "teacher" model, transferring "dark knowledge" Hinton calls inter-class similarities ([[https://arxiv.org/abs/1503.02531][Hinton 2015]]).

:ZH:
通过让"学生"小模型学习"教师"大模型 softmax 概率分布的模型压缩技术。
Hinton 称类间相似性中的隐含信息为"暗知识"
（[[https://arxiv.org/abs/1503.02531][Hinton 2015]]）。
:END:

** Core Mechanism 核心机制

The teacher's softmax distribution contains rich inter-class relationship information.

:ZH:
Teacher 模型输出的 softmax 概率分布包含丰富的类间关系信息。
:END:

#+begin_src python
soft_target = softmax(teacher_logits / T)
loss = T**2 * KL_divergence(soft_target, soft_prediction)
#+end_src

** Related Concepts 相关概念

- [[id:<UUID-from-roam-search>][Model Compression 模型压缩]]
- [[id:<UUID-from-roam-search>][Soft Labels 软标签]]
- [[id:<UUID-from-roam-search>][Temperature Scaling 温度缩放]]

* References

- [[https://arxiv.org/abs/1503.02531][Hinton et al. 2015: Distilling the Knowledge in a Neural Network]] | [[https://archive.today/submit/?url=https%3A%2F%2Farxiv.org%2Fabs%2F1503.02531][archive]]
- [[https://en.wikipedia.org/wiki/Knowledge_distillation][Wikipedia: Knowledge distillation]] | [[https://archive.today/submit/?url=https%3A%2F%2Fen.wikipedia.org%2Fwiki%2FKnowledge_distillation][archive]]
```

## 不变量（违反即拒绝创建）

1. 双语标题（英文在前，中文在后），FILETAGS 双语，**散文段落和列表项**有 `:ZH:` drawer 折叠中文（code blocks 和 References 不需要）
2. 至少 2 条 References，每条带 archive.today 链接
3. AI 生成笔记必须有 `:GENERATOR:`、`:MODEL:`、`:GENERATED_AT:` 三个 property + `AI_Generated`/`AI生成` 双语标签
4. **创建前**必须 `roam_search_title` 查重；**创建后**必须按"6. 织入知识图谱"调 `roam_search_title`/`roam_search_tag` 找相关节点 + `roam_create_link` 建立双向链接
5. References 中的 URL 必须真实（用 `WebSearch`/`fetch` 验证），禁止编造
