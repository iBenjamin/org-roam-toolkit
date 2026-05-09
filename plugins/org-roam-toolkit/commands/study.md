---
description: Study an article deeply by saving a reference note, creating atomic notes, and linking the graph
argument-hint: <url>
---

The user wants to study this article deeply. Complete two jobs:
1. Distill the article into a reference note.
2. Extract atomic concepts from the article, create standalone notes, and link them into the knowledge graph.

URL: $ARGUMENTS

**Format, References, AI marking, and graph-linking rules**: follow the `atomic-notes` skill.

## Step 1: Fetch the article

Use the `fetch` skill to fetch the full article. WeChat articles automatically use the WeChat site strategy.

## Step 2: Create the reference note

Distill the article:
- Extract core concepts and key insights.
- Restructure into org-mode format with headings, code blocks, tables, and other useful structure.
- Preserve important code examples and data.
- Remove fluff and redundancy while keeping the substance.

Call `roam_create_note`:
- `title`: original article title. Do not rewrite or polish it.
- `subdirectory`: `"reference"`.
- `sourceUrl`: original URL.
- `tags`: classification tags plus AI-generated tag, following `atomic-notes`.
- `properties`: follow the "AI-Generated Content Marking" section in `atomic-notes`.
- `content`: distilled org-mode content. Reference notes should preserve the source language when that matters; do not force translation.

## Step 3: Identify atomic concepts

Identify core concepts worth standalone notes:
- Each concept must be an independent knowledge unit that still makes sense outside the source article.
- Prefer fewer, higher-value concepts. Usually 2-5 concepts.
- A concept should be explainable with one essential sentence plus a few supporting points.

## Step 4: Check duplicates and create atomic notes

For each identified concept:

1. Run `roam_search_title`.
2. Existing note: skip creation, but record it for later linking.
3. Missing note: call `roam_create_note`:
   - `subdirectory`: `"main"`.
   - `tags`: classification tags plus AI-generated tag.
   - Follow all `atomic-notes` rules: English title, English prose, two-layer References, and AI marking.

## Step 5: Link into the knowledge graph

Follow the "Graph-Linking Rules" section in `atomic-notes`:
1. Each atomic note <-> reference note.
2. Related atomic notes <-> each other.
3. Use `roam_search_title` / `roam_search_tag` to find and link existing related notes.

## Step 6: Submit to archive.today

Collect all Reference URLs and follow the "Submit to archive.today" section in `atomic-notes`.

## Step 7: Return a summary

- Reference note path.
- Newly created atomic notes, including title and path.
- Links created.
- Existing notes skipped, if any.
