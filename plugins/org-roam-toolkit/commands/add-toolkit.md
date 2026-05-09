---
description: Add a tool or resource to the quarterly toolkit collection
argument-hint: <name> <url>
---

The user wants to add a tool, library, service, or API to the toolkit collection. The entry goes into quarterly files under `toolkit/YYYY-QN.org`; it is NOT an org-roam node.

Input: $ARGUMENTS

## Steps

1. Parse arguments: first token is `name`, second token is `url`, and the remaining text is the description.
2. If a URL is present, use `WebFetch` to verify it is reachable and fetch a one-sentence description when the user did not provide one.
3. Call the `roam_add_toolkit` MCP tool:
   - `title`: resource name.
   - `url`: resource URL.
   - `tags`: classification tags. See the table below.
   - `category`: `"library"` / `"tool"` / `"service"` / `"api"`.
   - `description`: one-sentence description.
4. Confirm the entry was added and return its location in the quarterly file.

## Tag System

| Dimension | Example tags |
|-----------|--------------|
| Type | `library`, `tool`, `service`, `api` |
| Domain | `ios`, `macos`, `web`, `design`, `marketing`, `ai` |
| Special | `free`, `paid`, `starred` |

## Examples

| Input | Parsed result |
|---|---|
| `SnapKit https://github.com/SnapKit/SnapKit iOS Auto Layout DSL` | name=SnapKit, url=..., description=iOS Auto Layout DSL, category=library, tags=[library, ios, ui] |
| `Raycast https://www.raycast.com/` | name=Raycast, url=..., fetched description, category=tool, tags=[tool, macos, productivity] |
| `OpenAI API https://platform.openai.com/` | category=api, tags=[api, ai, llm] |
