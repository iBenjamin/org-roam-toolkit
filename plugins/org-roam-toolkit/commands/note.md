---
description: Create an atomic concept note and link it into the knowledge graph
argument-hint: <concept name>
---

The user wants to create an atomic note for a concept.

Concept: $ARGUMENTS

**Format, References, AI marking, and graph-linking rules**: follow the `atomic-notes` skill. This command only defines the workflow.

## Step 1: Check for duplicates

Call `roam_search_title` to check whether the same or a near-same note already exists.
- If it exists: tell the user and ask whether to supplement/update it. Do not overwrite automatically.
- If it does not exist: continue.

## Step 2: Create the atomic note

Call `roam_create_note`:
- `title`: follow the English title format in `atomic-notes`.
- `subdirectory`: `"main"`.
- `tags`: classification tags.
- `content`: follow the English content format in `atomic-notes`, including the two-layer References structure.
- Mark AI-generated content using the Properties + FILETAGS rules in `atomic-notes`.

References must be real. Use `WebSearch` to find at least 2-3 authoritative sources for the concept.

## Step 3: Link into the knowledge graph

Follow the "Graph-Linking Rules" section in `atomic-notes`:
1. Use `roam_search_title` / `roam_search_tag` to find related existing notes.
2. Use `roam_create_link` to create bidirectional links.

## Step 4: Submit to archive.today

Follow the "Submit to archive.today" section in `atomic-notes`: collect every original URL from References and open each archive.today submission page with `emacsclient -e '(browse-url "...")'`, waiting one second between URLs.

## Step 5: Return a summary

- Note path.
- Links created, including count and important target node names.
