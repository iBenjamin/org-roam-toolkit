---
description: Add a read article to reading history
argument-hint: <url>
---

The user has already read this article and wants to add it to reading history. The entry goes into quarterly files under `read_history/YYYY-QN.org`; it is NOT an org-roam node.

URL: $ARGUMENTS

## Steps

1. Use the `fetch` skill to fetch the article. WeChat articles automatically use the WeChat site strategy.
2. Call the `roam_add_reading_history` MCP tool:
   - `title`: article title.
   - `url`: `$ARGUMENTS`.
   - `tags`: classification tags.
   - `source`: website name, for example `cnblogs`, `github`, or `weixin`.
   - `author`: author name, if available.
   - `summary`: one-sentence summary. If the article is empty, such as an image-only article, omit `summary`.
   - `points`: list of 2-5 key points.
   - `rating`: 1-5 rating, optional.
3. Confirm the entry was added and return its location in the quarterly file.
