---
name: atomic-notes
description: |
  Use when commands create or modify org-roam atomic notes, reference notes, or note networks.
  Covers English-first titles, tags, prose structure, References, archive.today links,
  AI-generated content marking, duplicate checks, and knowledge-graph linking.

  Triggers: atomic note, concept note, Zettelkasten, roam note, org-roam,
  FILETAGS, References, archive.today, ai_generated, backlink, knowledge graph
---

# Atomic Notes Skill

This skill defines the org-roam note format, source citation rules, AI-content marking, and graph-linking requirements used by `/note`, `/study`, `/deep_note`, `/reference`, and `/ref-extract`.

## 1. Title Format

Use concise English titles.

Format: `English Concept Name (ABBR)`

Rules:
- Put the canonical English name first.
- Include a common abbreviation at the end in parentheses.
- Do not add translated aliases unless the user explicitly asks for them.
- For people, products, standards, or proper nouns, keep the original official name.

Examples:
- `Knowledge Distillation (KD)`
- `Backpropagation (BP)`
- `Model Watermarking`
- `Attention Mechanism`
- `Remote Procedure Call (RPC)`

## 2. Tag Format

`#+FILETAGS` uses English tags only.

Format: `#+FILETAGS: :English_Tag:Another_Tag:`

Rules:
- Use English tags with underscores for multi-word terms.
- Preserve common abbreviations when useful: `:Intellectual_Property:IP:`
- Keep tags short, reusable, and category-like.

Examples:
- `#+FILETAGS: :AI_Security:Model_Attack:`
- `#+FILETAGS: :Machine_Learning:Model_Compression:`
- `#+FILETAGS: :Cryptography:Authentication:`

### When Calling `roam_create_note`

The `tags` array follows the same English-only convention:

```jsonc
// Correct: reusable English tags
"tags": ["AI_Security", "Model_Attack"]

// Wrong: sentence-like or one-off tags
"tags": ["this_is_about_security_attacks_on_models"]
```

## 3. Content Format

Core principle: write the main note in clear English prose. Add translated/local-language drawers only when the user explicitly requests them.

### Structure

```org
English one-line definition.

** Core Mechanism

English paragraph content.

** Related Concepts

- *English Term*: English description
- *English Term*: English description
```

### Rules

1. Start with a one-sentence definition.
2. Use level-2 headings for major sections.
3. Write complete paragraphs instead of fragments.
4. Lists should stay short and grouped under a descriptive heading.
5. Do not translate code blocks.
6. Do not translate References.
7. If the user asks for a localized version, put it in a drawer named after the locale, for example `:TRANSLATION:` or `:LOCAL_NOTE:`.

## 4. References Format

External citations use two layers.

### Layer 1: Inline Citations

When a sentence relies on a specific source, attach an org-mode link near the claim:

```org
Google's 2024 core updates targeted scaled low-quality AI-generated content ([[https://blog.google/...][Google Blog]]).
```

### Layer 2: References Section

Add a `* References` section at the end. Include every external source used in the note, and include an archive.today submission link for each URL:

```org
* References

- [[https://blog.google/...][Google Blog: Fighting spam]] | [[https://archive.today/submit/?url=https%3A%2F%2Fblog.google%2F...][archive]]
- [[https://example.com][Source Title]] | [[https://archive.today/submit/?url=https%3A%2F%2Fexample.com][archive]]
```

Requirements:
- archive.today link format: `https://archive.today/submit/?url=` plus the URL-encoded original URL.
- Provide at least 2-3 external sources unless the concept is unusually narrow.
- Keep inline citations and the final References section consistent.
- URLs must be real. Use WebSearch/fetch to verify authoritative sources. Never invent URLs.

### Submit to archive.today

After creating the note, collect every original URL from inline citations and References, excluding archive.today links, then open archive.today submission pages via emacsclient:

```bash
emacsclient -e '(browse-url "https://archive.today/submit/?url=<URL-encoded-original-url>")'
```

Wait one second between URLs with `sleep 1` to avoid overwhelming the browser.

## 5. AI-Generated Content Marking

All AI-generated notes MUST be clearly marked so users can distinguish generated material from human-written notes.

### Marking Rules

1. **Tag**: add `:AI_Generated:` to FILETAGS.
2. **Properties drawer**:
   - `GENERATOR`: AI system name, for example `claude`
   - `MODEL`: model version, for example `opus-4.7`
   - `GENERATED_AT`: generation timestamp, for example `[2026-05-08 Wed]`

### Complete Example

```org
#+FILETAGS: :AI_Generated:Cryptography:
:PROPERTIES:
:ID:           xxxx-xxxx
:GENERATOR:    claude
:MODEL:        opus-4.7
:GENERATED_AT: [2026-05-08 Wed]
:END:
#+title: Knowledge Distillation (KD)

A model compression technique where a smaller "student" model learns to mimic a larger "teacher" model.
```

### Scope

- Atomic notes created by `/note`, `/deep_note`, and `/ref-extract`: mark as AI-generated.
- Reference notes created by `/study` and `/reference`: always mark as AI-generated.
- Notes edited manually by the user through emacsclient: do not mark automatically.

## 6. Graph-Linking Rules

Every new note must actively search for existing related notes and link into the graph.

### Workflow

1. **Find related nodes**: use `roam_search_title` and `roam_search_tag`.
2. **Create bidirectional links**: use `roam_create_link` with `bidirectional: true`.
3. **Cross-link batches**: when creating multiple notes, link related concepts to each other too.

### First-Degree Related Concepts

Use these criteria for `/deep_note` and `/ref-extract`:

- Required prerequisite knowledge for understanding the core concept.
- A core component of the concept.
- An important contrast or neighboring concept.
- Keep the list to 3-7 concepts; prefer fewer high-value links.

### Skip Conditions

- Existing same or near-same note: skip creation, but still create links.
- Overly broad concepts such as "Architecture" or "Design Patterns": skip creation unless the user explicitly asks for that broad note.

### On-disk layout produced by `roam_create_link`

The MCP tool appends each inserted link as a `[[id:UUID][Title]]` paragraph beneath a top-level `* Links` heading at the end of the file. The heading is created automatically on the first link insertion and reused for subsequent inserts; the function is idempotent on the heading.

The author-written `** Related Concepts` section in the body is the prose-level semantic description and remains the human-authored layer. `* Links` is the machine-maintained edge list. Both coexist by design — they serve different readers.

The function deduplicates the heading but **not** the link paragraphs. Calling `roam_create_link A→B` twice produces two `[[id:B]]` paragraphs under `* Links`. Callers are responsible for not inserting duplicates.

Example tail of a finished note:

```org
* References

- [[https://example.com/spec][Spec]] | [[https://archive.today/...][archive]]

* Links

[[id:abcdef12-...][Related Concept A]]

[[id:fedcba21-...][Related Concept B]]
```

## 7. Complete Example

```org
:PROPERTIES:
:ID:           a1b2c3d4-e5f6-7890-abcd-ef1234567890
:GENERATOR:    claude
:MODEL:        opus-4.7
:GENERATED_AT: [2026-05-08 Wed]
:END:
#+FILETAGS: :AI_Generated:Machine_Learning:Model_Compression:
#+title: Knowledge Distillation (KD)

A model compression technique where a smaller "student" model learns to mimic the soft probability distributions of a larger "teacher" model, transferring the "dark knowledge" Hinton calls inter-class similarities ([[https://arxiv.org/abs/1503.02531][Hinton 2015]]).

** Core Mechanism

The teacher's softmax distribution contains rich inter-class relationship information.

#+begin_src python
soft_target = softmax(teacher_logits / T)
loss = T**2 * KL_divergence(soft_target, soft_prediction)
#+end_src

** Related Concepts

- [[id:<UUID-from-roam-search>][Model Compression]]
- [[id:<UUID-from-roam-search>][Soft Labels]]
- [[id:<UUID-from-roam-search>][Temperature Scaling]]

* References

- [[https://arxiv.org/abs/1503.02531][Hinton et al. 2015: Distilling the Knowledge in a Neural Network]] | [[https://archive.today/submit/?url=https%3A%2F%2Farxiv.org%2Fabs%2F1503.02531][archive]]
- [[https://en.wikipedia.org/wiki/Knowledge_distillation][Wikipedia: Knowledge distillation]] | [[https://archive.today/submit/?url=https%3A%2F%2Fen.wikipedia.org%2Fwiki%2FKnowledge_distillation][archive]]
```

## Invariants

1. English title, English FILETAGS, clear English prose, and no translated drawers unless explicitly requested.
2. At least 2 References, each with an archive.today submission link.
3. AI-generated notes must include `:GENERATOR:`, `:MODEL:`, `:GENERATED_AT:`, and the `AI_Generated` tag.
4. Before creating a note, run `roam_search_title` for duplicates. After creation, use `roam_search_title`/`roam_search_tag` and `roam_create_link` to create bidirectional links.
5. Reference URLs must be real and verified with `WebSearch`/`fetch`. Never fabricate URLs.
