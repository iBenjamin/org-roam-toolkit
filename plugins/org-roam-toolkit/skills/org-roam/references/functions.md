# Function Reference

Detailed documentation for all org-roam-skill functions.

## Table of Contents

- [Note Creation](#note-creation)
- [Search Functions](#search-functions)
- [Link Management](#link-management)
- [Tag Management](#tag-management)
- [Attachment Management](#attachment-management)
- [Utility Functions](#utility-functions)
- [Diagnostic Functions](#diagnostic-functions)

## Note Creation

### org-roam-skill-create-note

Create a new org-roam note with auto-detection of template format.

**Signature**: `(org-roam-skill-create-note TITLE &key tags properties content content-file keep-file subdirectory)`

**Parameters:**
- `TITLE` (string, required): The note title
- `:tags` (list of strings, optional): Tags as `'("tag1" "tag2")` - **MUST be a list**
- `:properties` (alist, optional): Additional properties for PROPERTIES drawer as `'(("KEY" . "value"))`
- `:content` (string, optional): Initial content (for small/simple content)
- `:content-file` (string, optional): Path to file containing content (for large content)
- `:keep-file` (boolean, optional): If `t`, prevent automatic deletion of `:content-file`
- `:subdirectory` (string, optional): Subdirectory within org-roam-directory (e.g., `"main"`, `"reference"`, `"projects"`, `"daily"`)
- `:source-url` (string, optional): Original URL for reference notes. Automatically appends a References section with original link and archive.today submission link
- `:open-archive` (boolean, optional): If `t`, automatically opens archive.today submission URL in browser after creating note (requires `:source-url`)

**Examples:**

Basic note:
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-create-note \"My Note\")"
```

With tags and content:
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-create-note \"React Hooks\" :tags '(\"javascript\" \"react\") :content \"Notes about hooks\")"
```

With AI-generated marking (tags + properties):
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-create-note \"AI Note\" :tags '(\"ai_generated\" \"topic\") :properties '((\"GENERATOR\" . \"claude\") (\"MODEL\" . \"opus-4.5\") (\"GENERATED_AT\" . \"[2026-01-13 Mon]\")) :content \"Content here\")"
```

Large content via file:
```bash
TEMP=$(mktemp -t org-roam-content.XXXXXX)
cat > "$TEMP" << 'EOF'
* Section 1
Content here

* Section 2
More content
EOF
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-create-note \"Large Note\" :content-file \"$TEMP\")"
# Temp file automatically deleted
```

With subdirectory:
```bash
# Create in reference directory
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-create-note \"Wikipedia: Linux\" :subdirectory \"reference\" :tags '(\"reference\"))"

# Create in projects directory
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-create-note \"Project Alpha Status\" :subdirectory \"projects\")"
```

With source URL (for reference notes):
```bash
# Creates note with auto-generated References section and opens browser
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-create-note \"Git Worktree\" :subdirectory \"reference\" :source-url \"https://example.com/git-worktree\" :open-archive t :tags '(\"git\") :content \"* Summary\\n\\nContent here...\")"
```

The generated References section:
```org
* References

- Git Worktree: [[https://example.com/git-worktree][original]] | [[https://archive.today/submit/?url=...][submit archive]]
```

With `:open-archive t`, browser automatically opens to archive.today submission page.

**Content Format:**

Content should be in org-mode format. For markdown conversion or general org-mode formatting, use the `orgmode` skill.

Example workflow:
```bash
# Step 1: Convert markdown to org (orgmode skill)
# Step 2: Create roam note with org content (this skill)
ortk-emacs-eval --pkg=org-roam-skill \
  "(org-roam-skill-create-note \"Title\" :content \"* Org heading\")"
```

**Automatic Behaviors:**
- Auto-detects filename format from `org-roam-capture-templates`
- Generates proper filenames (timestamp-only, timestamp-slug, or custom)
- Handles head content to avoid #+title duplication
- Sanitizes tags (replaces hyphens with underscores)
- Returns file path of created note

**Common tag mistakes:**
- ❌ `"planning"` (string)
- ✅ `'("planning")` (list with one element)
- ❌ `'planning` (unquoted symbol)
- ✅ `'("tag1" "tag2")` (list with multiple elements)

## Search Functions

### org-roam-skill-search-by-title

Search notes by title (case-insensitive, partial match).

**Signature**: `(org-roam-skill-search-by-title SEARCH-TERM)`

**Example:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-search-by-title \"react\")"
```

**Returns**: List of `(id title file)` tuples.

### org-roam-skill-search-by-tag

Search notes by tag.

**Signature**: `(org-roam-skill-search-by-tag TAG)`

**Example:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-search-by-tag \"javascript\")"
```

**Returns**: List of `(id title file)` tuples.

### org-roam-skill-search-by-content

Search notes by content (full-text search).

**Signature**: `(org-roam-skill-search-by-content SEARCH-TERM)`

**Example:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-search-by-content \"functional programming\")"
```

**Returns**: List of `(id title file)` tuples with matching content.

## Link Management

### org-roam-skill-get-backlinks-by-title

Find notes that link TO the specified note.

**Signature**: `(org-roam-skill-get-backlinks-by-title TITLE)`

**Example:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-get-backlinks-by-title \"React\")"
```

**Returns**: List of `(id title file)` tuples for notes linking to this note.

### org-roam-skill-get-backlinks-by-id

Find notes that link TO the specified note (by ID).

**Signature**: `(org-roam-skill-get-backlinks-by-id NODE-ID)`

**Example:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-get-backlinks-by-id \"abc123-def456\")"
```

### org-roam-skill-create-bidirectional-link

Create links between two notes (both directions).

**Signature**: `(org-roam-skill-create-bidirectional-link TITLE-A TITLE-B)`

**Example:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-create-bidirectional-link \"React Hooks\" \"React\")"
```

Creates:
- Link in "React Hooks" pointing to "React"
- Link in "React" pointing to "React Hooks"

### org-roam-skill-insert-link-in-note

Insert a link in one note pointing to another.

**Signature**: `(org-roam-skill-insert-link-in-note SOURCE-TITLE TARGET-TITLE)`

**Example:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-insert-link-in-note \"My Note\" \"React\")"
```

Adds link to "React" at the end of "My Note".

## Tag Management

### org-roam-skill-list-all-tags

List all unique tags across all notes.

**Signature**: `(org-roam-skill-list-all-tags)`

**Example:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-list-all-tags)"
```

**Returns**: Sorted list of all unique tags.

### org-roam-skill-add-tag

Add a tag to a note.

**Signature**: `(org-roam-skill-add-tag TITLE TAG)`

**Example:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-add-tag \"My Note\" \"important\")"
```

### org-roam-skill-remove-tag

Remove a tag from a note.

**Signature**: `(org-roam-skill-remove-tag TITLE TAG)`

**Example:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-remove-tag \"My Note\" \"draft\")"
```

## Attachment Management

### org-roam-skill-attach-file

Attach a file to a note (copies file to attachment directory).

**Signature**: `(org-roam-skill-attach-file TITLE FILE-PATH)`

**Example:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-attach-file \"My Note\" \"/path/to/document.pdf\")"
```

**Behavior:**
- Copies file to `{org-attach-id-dir}/{node-id}/filename`
- Adds `ATTACH` property to note automatically
- Uses org-mode's standard `org-attach` system

### org-roam-skill-list-attachments

List all attachments for a note.

**Signature**: `(org-roam-skill-list-attachments TITLE)`

**Example:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-list-attachments \"My Note\")"
```

**Returns**: List of attachment filenames.

### get-attachment-path

Get full path to a specific attachment.

**Signature**: `(get-attachment-path TITLE FILENAME)`

**Example:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(get-attachment-path \"My Note\" \"document.pdf\")"
```

### delete-note-attachment

Delete an attachment from a note.

**Signature**: `(delete-note-attachment TITLE FILENAME)`

**Example:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(delete-note-attachment \"My Note\" \"old-file.pdf\")"
```

### get-note-attachment-dir

Get the attachment directory path for a note.

**Signature**: `(get-note-attachment-dir TITLE)`

**Example:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(get-note-attachment-dir \"My Note\")"
```

**Returns**: Path to note's attachment directory.

## Utility Functions

### org-roam-skill-check-setup

Check if org-roam is properly configured.

**Signature**: `(org-roam-skill-check-setup)`

**Example:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-check-setup)"
```

**Returns**: Status message about setup (directory exists, database initialized, etc.).

### org-roam-skill-get-graph-stats

Get statistics about the knowledge graph.

**Signature**: `(org-roam-skill-get-graph-stats)`

**Example:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-get-graph-stats)"
```

**Returns**: Statistics like total notes, total links, tags count, etc.

### org-roam-skill-find-orphan-notes

Find notes with no backlinks or forward links.

**Signature**: `(org-roam-skill-find-orphan-notes)`

**Example:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-skill-find-orphan-notes)"
```

**Returns**: List of `(id title file)` tuples for orphaned notes.

## Diagnostic Functions

### org-roam-doctor

Comprehensive diagnostic check of org-roam setup.

**Signature**: `(org-roam-doctor)`

**Example:**
```bash
ortk-emacs-eval --pkg=org-roam-skill "(org-roam-doctor)"
```

**Checks:**
- Emacs version
- org-roam version
- org-roam directory exists and is accessible
- Database location and status
- Capture templates configuration
- Database schema version

**Returns**: Detailed diagnostic report.

## Parsing emacsclient Output

emacsclient returns Elisp-formatted data:

- **Strings**: `"result"` (with quotes)
- **Lists**: `("item1" "item2" "item3")`
- **nil**: No output or `nil`
- **Numbers**: `42`

You may need to:
- Strip surrounding quotes from strings
- Parse list structures
- Handle nil/empty results

## Best Practices

1. Use `org-roam-node-*` functions for data access
2. Use `org-roam-node-from-title-or-alias` for flexible searching
3. Always check if nodes exist before operations
4. Sync database after creating/modifying notes if needed
5. Leverage org-roam's query functions rather than SQL directly
6. Use `seq-filter` and `mapcar` for list operations
7. Use `:content-file` for large content (automatic cleanup)
8. Always use lists for tags: `'("tag1" "tag2")`
