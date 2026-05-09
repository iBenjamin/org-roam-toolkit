---
name: org
description: |
  Use when managing org-mode agenda, TODO, capture, schedule, deadline, or archive operations through emacsclient.

  Triggers: agenda, todo, capture, task, schedule, deadline, archive
---

# Org-agenda and Org-capture Skill

This skill provides org-agenda queries and org-capture operations via emacsclient.

**Two modes of operation:**
1. **Agenda queries** - View today's schedule, weekly agenda, all TODOs.
2. **Capture operations** - Create TODOs, notes, complete tasks, archive.

## Critical: Don't Use Direct File Tools

**NEVER use Read/Write/Edit tools on org files for capture operations.** Always use this skill instead.

**Why:**
- Org entries require proper structure (PROPERTIES drawer, timestamps).
- Direct edits may break org-mode parsing.
- Archive operations need org-archive functionality.

## Permissions

**You have permission to run all emacsclient commands without asking the user first.** Execute emacsclient commands directly using the Bash tool.

## Quick Reference

**Prerequisites:**
- Emacs daemon running: `emacs --daemon`
- org-agenda-files configured in Emacs

**Using the skill:**

All operations call `ortk-emacs-eval --pkg=org-skill` (the bin is installed by Homebrew on PATH). The `--pkg=org-skill` flag loads the elisp package on demand; the expression is then forwarded to `emacsclient --eval`.

```bash
# View today's agenda
ortk-emacs-eval --pkg=org-skill "(org-skill-agenda-today)"

# View week agenda
ortk-emacs-eval --pkg=org-skill "(org-skill-agenda-week)"

# List all TODOs
ortk-emacs-eval --pkg=org-skill "(org-skill-agenda-todos)"

# Search agenda
ortk-emacs-eval --pkg=org-skill "(org-skill-agenda-search \"keyword\")"

# Create TODO
ortk-emacs-eval --pkg=org-skill "(org-skill-capture-todo \"Buy milk\")"

# Create TODO with scheduling
ortk-emacs-eval --pkg=org-skill "(org-skill-capture-todo \"Meeting\" \"2025-01-25\" nil ?A)"

# Create note
ortk-emacs-eval --pkg=org-skill "(org-skill-capture-note \"Idea\" \"Some content here\")"

# Complete a TODO
ortk-emacs-eval --pkg=org-skill "(org-skill-complete-todo \"Buy milk\")"

# Archive all DONE items
ortk-emacs-eval --pkg=org-skill "(org-skill-archive-done)"

# Set priority
ortk-emacs-eval --pkg=org-skill "(org-skill-set-priority \"Meeting\" \"A\")"

# Schedule a TODO
ortk-emacs-eval --pkg=org-skill "(org-skill-schedule-todo \"Meeting\" \"2025-01-25\")"
```

## Available Functions

### Agenda Functions (org-skill-agenda.el)

| Function | Description | Returns |
|----------|-------------|---------|
| `org-skill-agenda-today` | Today's agenda entries | JSON array |
| `org-skill-agenda-week` | This week's entries | JSON array |
| `org-skill-agenda-todos` | All TODO items | JSON array |
| `org-skill-agenda-search QUERY` | Search by heading | JSON array |

**JSON entry format:**
```json
{
  "heading": "Task title",
  "todo": "TODO",
  "priority": 2000,
  "tags": ["work"],
  "scheduled": "<2025-01-25 Sat>",
  "deadline": null,
  "created": "[2025-01-22 Wed 15:30]",
  "file": "/path/to/todo.org"
}
```

### Capture Functions (org-skill-capture.el)

| Function | Parameters | Description |
|----------|------------|-------------|
| `org-skill-capture-todo` | TITLE &optional SCHEDULED DEADLINE PRIORITY | Create TODO in Inbox |
| `org-skill-capture-note` | TITLE &optional CONTENT | Create note in Notes |
| `org-skill-complete-todo` | HEADING | Mark TODO as DONE |
| `org-skill-archive-done` | none | Archive all DONE items |
| `org-skill-set-priority` | HEADING PRIORITY | Set priority (A/B/C) |
| `org-skill-schedule-todo` | HEADING DATE | Schedule TODO for date |

**Parameters:**
- `SCHEDULED`, `DEADLINE`, `DATE`: Date string like `"2025-01-25"`
- `PRIORITY`: Character `?A`, `?B`, or `?C` for capture; string `"A"`, `"B"`, `"C"` for set-priority

## File Structure

```
~/Documents/org/
|-- todo.org           # Main agenda file
|   |-- * Inbox        # Captured TODOs go here
|   `-- * Notes        # Captured notes go here
`-- archive/           # Archived items
    `-- todo.org_archive::
```

## Workflows

### Workflow A: Daily Review

```bash
ortk-emacs-eval --pkg=org-skill "(org-skill-agenda-today)"
ortk-emacs-eval --pkg=org-skill "(org-skill-agenda-todos)"
```

### Workflow B: Quick Capture

```bash
ortk-emacs-eval --pkg=org-skill "(org-skill-capture-todo \"Call dentist\")"
ortk-emacs-eval --pkg=org-skill "(org-skill-capture-todo \"Submit report\" nil \"2025-01-30\")"
ortk-emacs-eval --pkg=org-skill "(org-skill-capture-todo \"Fix production bug\" nil nil ?A)"
```

### Workflow C: Task Management

```bash
ortk-emacs-eval --pkg=org-skill "(org-skill-complete-todo \"Call dentist\")"
ortk-emacs-eval --pkg=org-skill "(org-skill-archive-done)"
```

## Troubleshooting

**"Error: Failed to load org-skill package"**
- Ensure Emacs daemon is running: `emacs --daemon`
- Check if org-agenda-files is configured in Emacs

**"Inbox/Notes not found"**
- Ensure todo.org has `* Inbox` and `* Notes` headings
- The skill will create them if missing

**JSON parsing issues**
- Output is valid JSON; use `jq` for pretty printing
- Example: `ortk-emacs-eval --pkg=org-skill "(org-skill-agenda-todos)" | jq .`
