---
name: org-roam
description: |
  Org-mode formatting and org-roam note management via emacsclient. Never use Read/Write/Edit on roam notes directly.

  Triggers: roam note, org-roam, org-mode, .org files, Zettelkasten, backlinks
---

# Org-mode and Org-roam Skill

This skill provides comprehensive org-mode knowledge and org-roam note management via emacsclient.

**Two modes of operation:**
1. **Org-mode formatting** — Reference docs for syntax, properties, timestamps, links (no emacsclient needed)
2. **Org-roam operations** — Create, search, link notes via MCP tools (preferred) or emacsclient

## Critical: Don't Use Direct File Tools

**NEVER use Read/Write/Edit tools on roam notes.** Always use MCP tools or skill commands instead.

**Why:**
- Roam notes require org-roam database updates
- IDs must be generated with microseconds precision
- File creation must respect user's capture templates
- Direct file operations bypass database sync and break backlinks

**Trigger patterns:**
- User mentions "roam note" or "org-roam"
- File paths contain `/roam/` or `/org-roam/`
- Keywords: backlinks, Zettelkasten, knowledge graph, PKM, second brain

## MCP Tools (Preferred)

Use these MCP tools directly for all org-roam operations. No Bash permissions needed.

**Available tools:**
| Tool | Description |
|------|-------------|
| `roam_create_note` | Create org-roam note |
| `roam_search_title` | Search by title |
| `roam_search_tag` | Search by tag |
| `roam_search_content` | Full-text search |
| `roam_get_backlinks` | Get backlinks for a note |
| `roam_create_link` | Create links between notes |
| `roam_add_reading_history` | Add to quarterly reading log |
| `roam_add_toolkit` | Add to quarterly toolkit |
| `roam_add_to_read` | Add TODO to read later |
| `roam_list_tags` | List all tags |
| `roam_doctor` | Run diagnostics |

**Quick examples:**

```
# Create note
roam_create_note(title="My Note", tags=["topic"], content="content here")

# Create reference note with source URL
roam_create_note(title="Article", subdirectory="reference", sourceUrl="https://...")

# Search
roam_search_title(query="react")
roam_search_tag(tag="javascript")

# Links
roam_get_backlinks(title="React")
roam_create_link(source="React Hooks", target="React", bidirectional=true)

# Reading history (NOT org-roam node)
roam_add_reading_history(title="Article", url="https://...", tags=["topic"], summary="...")

# Toolkit (NOT org-roam node)
roam_add_toolkit(title="SnapKit", url="https://...", category="library")
```

## Fallback: Bash Commands

If MCP tools are unavailable, fall back to `ortk-emacs-eval --pkg=org-roam-skill` (installed by Homebrew on PATH):

```bash
# Create note (tags MUST be a list, not string)
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-create-note \"Title\" :tags '(\"tag\") :content \"text\")"

# Create with large content (recommended for >1KB content)
TEMP=$(mktemp -t org-roam-content.XXXXXX)
echo "Large content..." > "$TEMP"
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-create-note \"Title\" :content-file \"$TEMP\")"
# Temp file auto-deleted!

# Search
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-search-by-title \"search-term\")"

# Backlinks
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-get-backlinks-by-title \"Note Title\")"

# Link notes
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-create-bidirectional-link \"Note A\" \"Note B\")"

# Attach file
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-attach-file \"Note Title\" \"/path/to/file\")"

# Diagnostics
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-doctor)"
```

**Key principle**: Package auto-loads on first call, then stays in memory - no repeated loading overhead.

## Directory Classification

Notes are organized into subdirectories within `org-roam-directory`:

| Directory | Use Case | Examples |
|-----------|----------|----------|
| `daily` | Daily logs, journals, fleeting thoughts | 今日计划、随想、会议记录 |
| `reference` | External sources, articles, docs | Wikipedia 摘要、新闻、API 文档、教程 |
| `projects` | Project-specific notes | 项目名相关、任务跟踪、进度记录 |
| `main` | Conceptual knowledge (default) | 技术原理、概念笔记、学习总结 |
| `read_history` | Reading log (NOT org-roam nodes) | 文章阅读记录，按季度文件组织 |
| `toolkit` | Tool/resource collection (NOT org-roam nodes) | 库、工具、服务、API 收藏 |

**Classification rules (Claude auto-selects):**
1. User explicitly requests a category → use that
2. User says "阅读历史/reading history/加入阅读" → use `org-roam-skill-add-reading-history` (NOT create-note)
3. User says "工具收藏/toolkit/收藏工具/加入toolkit" → use `org-roam-skill-add-toolkit-resource` (NOT create-note)
4. Content is from external URL → `reference`
5. User mentions "今天/today/日志/journal" → `daily`
6. User mentions specific project name → `projects`
7. Default → `main`

**Usage with `:subdirectory` parameter:**
```bash
# Default (main directory)
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-create-note \"DNS 记录类型\")"

# Explicit subdirectory
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-create-note \"项目A进度\" :subdirectory \"projects\")"
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-create-note \"Wikipedia: Linux\" :subdirectory \"reference\")"
```

## Core Workflows

### Reading History (NOT org-roam nodes)

Reading history is a consumption log organized by quarterly files (e.g., `2026-Q1.org`). Each article is a level-1 heading with properties. This is NOT the same as creating org-roam notes.

**Add reading history entry:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-add-reading-history \"Article Title\" \"https://example.com/article\" :tags '(\"topic\") :source \"website\" :summary \"One line summary\" :points '(\"Key point 1\" \"Key point 2\") :rating 4)"
```

**Parameters:**
- `title` (required): Article title
- `url` (required): Source URL
- `:tags`: List of classification tags
- `:source`: Website name (e.g., "cnblogs", "github", "weixin")
- `:summary`: One-line summary
- `:points`: List of key points
- `:rating`: 1-5 rating (optional)

**Result format in quarterly file:**
```org
* Article Title                                      :topic:
:PROPERTIES:
:URL:      https://example.com/article
:READ_AT:  [2026-01-22 Wed 15:40]
:SOURCE:   website
:RATING:   4
:END:

One line summary

- Key point 1
- Key point 2

[[https://example.com/article][原文]] | [[https://archive.today/submit/?url=...][存档]]
```

### Toolkit Resources (NOT org-roam nodes)

Toolkit is a resource collection organized by quarterly files (e.g., `2026-Q1.org`). Each resource is a level-1 heading with properties. This is NOT the same as creating org-roam notes.

**Add toolkit resource:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-add-toolkit-resource \"SnapKit\" \"https://github.com/SnapKit/SnapKit\" :tags '(\"library\" \"ios\" \"ui\") :category \"library\" :description \"iOS Auto Layout DSL\")"
```

**Parameters:**
- `title` (required): Resource name
- `url` (required): Resource URL
- `:tags`: List of classification tags
- `:category`: Resource type (library / tool / service / api)
- `:description`: One-line description

**Result format in quarterly file:**
```org
* SnapKit                                           :library:ios:ui:
:PROPERTIES:
:URL:      https://github.com/SnapKit/SnapKit
:CATEGORY: library
:FOUND_AT: 20260122
:END:

iOS Auto Layout DSL
```

### Source Management

When creating reference notes from external URLs, use `:source-url`:

```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-create-note \"Article Title\" :subdirectory \"reference\" :source-url \"https://example.com/article\" :content \"...\")"
```

This:
1. Generates a References section with original + archive links
2. **Automatically opens browser** to archive.today submission page (for reference notes)

```org
* References

- Article Title: [[https://example.com/article][original]] | [[https://archive.today/submit/?url=...][submit archive]]
```

**Workflow:**
1. Create note with `:source-url` in reference subdirectory → browser auto-opens archive.today
2. Complete captcha if needed, wait for archive
3. Replace "submit archive" link with actual archive URL

**`:open-archive` behavior:**
- `:default` (omitted): Auto-opens browser for `reference` subdirectory only
- `t`: Always open browser
- `nil`: Never open browser

### AI-Generated Content Marking

All AI-generated notes MUST be clearly marked to identify potential hallucinations:

1. **Tag**: Add `ai_generated` tag for quick filtering
2. **Properties**: Add metadata in PROPERTIES drawer:
   - `GENERATOR`: The AI system (e.g., `claude`)
   - `MODEL`: The model used (e.g., `opus-4.5`)
   - `GENERATED_AT`: Timestamp of generation

**Required format:**
```org
#+filetags: :ai_generated:
:PROPERTIES:
:ID: xxx
:GENERATOR: claude
:MODEL: opus-4.5
:GENERATED_AT: [2026-01-13 Mon]
:END:
#+title: Note Title
```

**Example with full marking:**
```bash
TEMP=$(mktemp -t org-roam-content.XXXXXX)
cat > "$TEMP" << 'EOF'
* Summary

AI-generated content here...

* References

- Source: [[https://example.com][original]] | [[https://archive.today/xxx][archive]]
EOF

ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-create-note \"Note Title\" :tags '(\"ai_generated\" \"topic\") :properties '((\"GENERATOR\" . \"claude\") (\"MODEL\" . \"opus-4.5\") (\"GENERATED_AT\" . \"[2026-01-13 Mon]\")) :content-file \"$TEMP\")"
```

**Important:** Never omit AI marking. Users must be able to distinguish AI-generated content from human-written notes.

### Workflow A: Creating Notes

**Simple note:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-create-note \"Note Title\")"
```

**With tags and content:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-create-note \"React Hooks\" :tags '(\"javascript\" \"react\") :content \"Brief notes here\")"
```

**With large content (recommended for complex/large content):**
```bash
# Create temp file
TEMP=$(mktemp -t org-roam-content.XXXXXX)

# Write content
cat > "$TEMP" << 'EOF'
* Introduction

Content here with proper org-mode formatting.

* Details

More content.
EOF

# Create note (temp file is automatically deleted)
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-create-note \"My Note\" :tags '(\"project\") :content-file \"$TEMP\")"
```

**Critical: Tags must be a list:**
- ❌ Wrong: `:tags "tag"` (string)
- ✅ Correct: `:tags '("tag")` (list)
- ✅ Correct: `:tags '("tag1" "tag2")` (multiple tags)

**Content format:**

Content should be in org-mode format. For markdown conversion or general org-mode formatting, use the `orgmode` skill:

```bash
# Example workflow:
# 1. Convert markdown to org (orgmode skill)
# 2. Create roam note with org content (this skill)
ortk-emacs-eval --pkg=org-roam-skill \
  "(org-roam-skill-create-note \"Title\" :content \"* Org content\")"
```

For general org-mode operations (formatting, conversion, validation), see the **orgmode** skill. This skill focuses on org-roam-specific operations: note creation, database sync, node linking, and graph management.

See **references/functions.md** for detailed parameter documentation.

### Workflow B: Searching Notes

**By title:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-search-by-title \"react\")"
```

**By tag:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-search-by-tag \"javascript\")"
```

**By content:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-search-by-content \"functional programming\")"
```

**List all tags:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-list-all-tags)"
```

### Workflow C: Managing Links

**Find backlinks (notes linking TO this note):**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-get-backlinks-by-title \"React\")"
```

**Create bidirectional links:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-create-bidirectional-link \"React Hooks\" \"React\")"
```

This creates:
- Link in "React Hooks" → "React"
- Link in "React" → "React Hooks"

**Insert one-way link:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-insert-link-in-note \"Source Note\" \"Target Note\")"
```

### Workflow D: File Attachments

**Attach file:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-attach-file \"My Note\" \"/path/to/document.pdf\")"
```

**List attachments:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-list-attachments \"My Note\")"
```

Attachments use org-mode's standard `org-attach` system.

### Workflow E: Complete Example

User says: "Create a note about React Hooks and link it to my React note"

**Step 1: Search for existing note**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-node-from-title-or-alias \"React\")"
```

**Step 2: Create new note**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-create-note \"React Hooks\" :tags '(\"javascript\" \"react\") :content \"Notes about React Hooks\")"
```

**Step 3: Create bidirectional links**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-create-bidirectional-link \"React Hooks\" \"React\")"
```

**Step 4: Show user the result**
Present the created note path and confirm links were established.

## Using the Auto-Load Wrapper

All operations use `ortk-emacs-eval --pkg=org-roam-skill` which:
1. Auto-loads `org-roam-skill` package on first call
2. Connects to running Emacs daemon
3. Executes the elisp expression

After first call, functions stay in memory - no loading overhead.

**Find org-roam directory:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "org-roam-directory"
```

**Sync database (if needed):**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-db-sync)"
```

## Available Functions

All functions use `org-roam-skill-` prefix:

**Note Management:**
- `org-roam-skill-create-note` - Create new org-roam notes
- `org-roam-skill-add-reading-history` - Add entry to quarterly reading log (NOT org-roam node)
- `org-roam-skill-add-toolkit-resource` - Add resource to quarterly toolkit (NOT org-roam node)
- `org-roam-skill-search-by-title/tag/content` - Search notes
- `org-roam-skill-get-backlinks-by-title/id` - Find backlinks
- `org-roam-skill-insert-link-in-note` - Insert links
- `org-roam-skill-create-bidirectional-link` - Create two-way links

**Tag Management:**
- `org-roam-skill-list-all-tags` - List all tags
- `org-roam-skill-add-tag` - Add tag to note
- `org-roam-skill-remove-tag` - Remove tag from note

**Attachments:**
- `org-roam-skill-attach-file` - Attach file to note
- `org-roam-skill-list-attachments` - List attachments

**Utilities:**
- `org-roam-skill-check-setup` - Verify configuration
- `org-roam-skill-get-graph-stats` - Graph statistics
- `org-roam-skill-find-orphan-notes` - Find isolated notes
- `org-roam-doctor` - Comprehensive diagnostics

See **references/functions.md** for complete function documentation with all parameters and examples.

## Setup and Troubleshooting

**Installation:** See **references/installation.md** for:
- Prerequisites (Emacs daemon, org-roam)
- No manual configuration needed (auto-loads on first use)
- Optional: org-roam configuration recommendations

**Troubleshooting:** See **references/troubleshooting.md** for:
- Connection issues (daemon not running)
- Package loading problems
- Database sync issues
- Tag formatting errors
- Search problems
- Link issues
- Performance optimization

**Quick diagnostic:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-doctor)"
```

## Parsing emacsclient Output

emacsclient returns Elisp-formatted data:
- Strings: `"result"` (with quotes)
- Lists: `("item1" "item2")`
- nil: `nil` or no output
- Numbers: `42`

Strip quotes from strings and parse structures as needed.

## Best Practices

1. **Use lists for tags**: Always `'("tag")` not `"tag"`
2. **Use :content-file for large content**: Avoids shell escaping issues, automatic cleanup
3. **Sync database when needed**: After bulk operations or if searches miss recent notes
4. **Use node IDs for reliable linking**: More stable than file paths
5. **Check if nodes exist**: Before operations on specific notes
6. **Present results clearly**: Format output for user readability
7. **Handle errors gracefully**: Check daemon running, packages loaded

## Additional Resources

**Org-mode Formatting (no emacsclient needed):**
- **org-syntax.md** - Complete org-mode syntax reference
- **properties.md** - Property drawers and node properties
- **timestamps.md** - Date/time formats, scheduling, deadlines
- **links.md** - Internal and external link syntax
- **examples.md** - Common formatting patterns

**Org-roam Operations (via emacsclient):**
- **emacsclient-usage.md** - Detailed emacsclient patterns
- **org-roam-api.md** - Org-roam API reference
- **functions.md** - Complete function documentation
- **installation.md** - Setup and configuration guide
- **troubleshooting.md** - Common issues and solutions

**Quick access patterns:**
- Need org-mode syntax? → `references/org-syntax.md`
- Working with timestamps? → `references/timestamps.md`
- Creating links? → `references/links.md`
- Need installation help? → `references/installation.md`
- Function parameters unclear? → `references/functions.md`
- Something not working? → `references/troubleshooting.md`
