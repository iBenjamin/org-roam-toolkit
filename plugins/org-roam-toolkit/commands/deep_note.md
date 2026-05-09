---
description: Create an atomic concept note plus first-degree related concept notes
argument-hint: <concept name>
---

The user wants to create a small atomic-note network for a concept and its directly related concepts.

Concept: $ARGUMENTS

**Format, References, AI marking, and graph-linking rules**: follow the `atomic-notes` skill. The criteria for first-degree related concepts are in its "Graph-Linking Rules" section.

## Step 1: Understand the core concept and identify first-degree related concepts

Use your understanding of the concept to identify:
1. The essential definition of the core concept.
2. The concept's first-degree related concepts, using the `atomic-notes` criteria. Aim for 3-7 high-value concepts.

## Step 2: Batch duplicate check

For the core concept and every first-degree related concept, call `roam_search_title`.
- Existing note: record it for later linking. Do not create a duplicate.
- Missing note: record it for creation.

## Step 3: Create the core concept note

Call `roam_create_note`:
- `title`, `subdirectory: "main"`, `tags`, and AI-marking properties: follow `atomic-notes`.
- `content`: follow `atomic-notes`, and additionally include:
  - One-sentence essential definition.
  - Expanded key points.
  - A "Related Concepts" section listing all first-degree related concepts with `[[id:xxx]]` links when IDs are known.
  - Important code or data examples when relevant.
  - References from authoritative sources found with WebSearch, at least 2-3 sources.

## Step 4: Create first-degree related concept notes

For each missing first-degree related concept, call `roam_create_note`:
- Use the same format as the core concept note.
- Do not recursively expand its own related-concepts section. This prevents unbounded note creation.

## Step 5: Link into the knowledge graph

Follow the "Graph-Linking Rules" section in `atomic-notes`:
1. Core concept <-> each first-degree related concept.
2. First-degree related concepts <-> each other when there is a meaningful relationship.
3. Use `roam_search_title` / `roam_search_tag` to find and link existing related notes.

## Step 6: Submit to archive.today

Follow the "Submit to archive.today" section in `atomic-notes`: collect Reference URLs from every new note and open each archive.today submission page with `browse-url`, waiting one second between URLs.

## Step 7: Return a summary

- Core concept note path.
- Newly created first-degree related concept notes.
- Existing notes skipped.
- Link graph created.
