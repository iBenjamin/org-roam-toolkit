---
description: Analyze a reference note and extract atomic concepts plus first-degree related concepts
argument-hint: <reference note title keywords>
---

The user wants to extract deeper knowledge from an existing reference note and create an atomic-note network.

Keywords: $ARGUMENTS

**Format, References, AI marking, and graph-linking rules**: follow the `atomic-notes` skill.

## Step 1: Locate the reference note

Call `roam_search_title` to search for the reference note:
- One match: continue.
- Multiple matches: list them and ask the user to choose.
- No match: ask the user to check the title keywords.

## Step 2: Read the reference content

Read the full note via emacsclient:

```bash
emacsclient -e '(with-temp-buffer (insert-file-contents "<file-path>") (buffer-string))'
```

## Step 3: Identify atomic concepts

Identify core concepts that deserve standalone notes:
- Each concept must be an independent knowledge unit that still makes sense outside the source article.
- Prefer fewer, higher-value concepts. Usually 2-5 concepts.
- A concept should be explainable with one essential sentence plus a few supporting points.
- Skip concepts that are too broad, such as "Architecture" or "Design Patterns".
- Skip notes that already exist after batch checking with `roam_search_title`.

## Step 4: Expand first-degree related concepts

For each atomic concept, identify 2-5 first-degree related concepts using the "Graph-Linking Rules" criteria in `atomic-notes`, then batch-check duplicates with `roam_search_title`.

## Step 5: Create atomic notes

For every missing atomic concept, call `roam_create_note`:
- `subdirectory: "main"`, `tags`, and AI-marking properties: follow `atomic-notes`.
- `content`: follow `atomic-notes`, and additionally include:
  - One-sentence essential definition.
  - Expanded key points.
  - A "Related Concepts" section with first-degree related concepts as `[[id:xxx]]` links when IDs are known.
  - Important code or data examples.
  - References from authoritative sources found with WebSearch, at least 2-3 sources.

## Step 6: Create first-degree related concept notes

For every missing first-degree related concept, call `roam_create_note`:
- Use the same format as above.
- Do not recursively expand its own related-concepts section.

## Step 7: Link into the knowledge graph

Follow the "Graph-Linking Rules" section in `atomic-notes`:
1. Each atomic note <-> source reference note.
2. Atomic note <-> its first-degree related concepts.
3. Atomic notes <-> each other when there is a meaningful relationship.
4. Use `roam_search_title` / `roam_search_tag` to find and link existing related notes.

## Step 8: Submit to archive.today

Follow the "Submit to archive.today" section in `atomic-notes`, opening each archive.today submission page with `browse-url` and waiting one second between URLs.

## Step 9: Return a summary

- Source reference note title and path.
- Extracted atomic concepts.
- Newly created atomic notes, including title and path.
- Newly created first-degree related concept notes.
- Existing notes skipped.
- Link graph created.
