---
description: Add a URL to the read-later list
argument-hint: <url>
---

The user wants to read this article later.

URL: $ARGUMENTS

## Steps

1. Use `WebFetch` to fetch the page, extract the title, and generate a one-sentence summary of 15 words or fewer.
2. Call the `roam_add_to_read` MCP tool:
   - `title`: article title.
   - `url`: `$ARGUMENTS`.
   - `summary`: one-sentence summary. Omit `summary` if content cannot be fetched.
3. Confirm the entry was added and return its location under the `todo.org` Inbox.
