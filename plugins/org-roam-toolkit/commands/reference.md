---
description: Save a valuable article or local file as an org-roam reference note
argument-hint: <url or local-file-path>
---

The user wants to save this article or file as an org-roam reference note.

Input: $ARGUMENTS

**AI marking rules**: follow the "AI-Generated Content Marking" section in `atomic-notes`. Reference notes are always marked as AI-generated when created by this command.

Determine the input type:
- Starts with `http://` or `https://`: URL workflow.
- Otherwise: local-file workflow.

---

## URL workflow

1. Use the `fetch` skill to fetch the article. WeChat articles automatically use the WeChat site strategy.
2. Extract the title and distill the article:
   - Extract core concepts and key insights.
   - Restructure into org-mode format with headings, code blocks, tables, and other useful structure.
   - Preserve important code examples and data.
   - Remove fluff and redundancy while keeping the substance.
3. Choose classification tags.
4. Call `roam_create_note`:
   - `title`: original article title. Do not rewrite or polish it.
   - `subdirectory`: `"reference"`.
   - `sourceUrl`: original URL.
   - `tags`: classification tags plus AI-generated tag.
   - `properties`: follow the AI-marking rules in `atomic-notes`.
   - `content`: distilled org-mode content.
5. Return the note path.

---

## Local-file workflow

Local files do not have a stable external source. The reference note becomes the persistent copy, so preserve the full content. Do not distill or omit content.

1. Use the `Read` tool to read the local file.
2. Extract a title. Use a top-level heading if present; otherwise use the filename.
3. Convert the full content to org-mode format. Convert markdown to org and do not remove any content.
4. Choose classification tags.
5. Call `roam_create_note`:
   - `title`: original article/file title.
   - `subdirectory`: `"reference"`.
   - `sourceUrl`: unset, because there is no external source.
   - `tags`: classification tags plus AI-generated tag.
   - `properties`: follow the AI-marking rules in `atomic-notes`, and add `LOCAL_SOURCE: <original-file-path>`.
   - `content`: full org-mode content.
6. Return the note path.
