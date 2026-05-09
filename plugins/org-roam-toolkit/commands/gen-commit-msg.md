---
description: Scan all changes, generate a commit message, commit, and push
---

Your task is to scan all changes, generate a commit message, stage everything, commit, and push with no confirmation needed.

> **Explicit-invocation contract.** The user invoking `/gen-commit-msg` is the explicit authorization to commit and push. This bypasses the default "don't push without explicit request" rule, which still applies in plain conversations. Use `git commit` directly there.

## Step 1: Scan

Run in parallel:
- `git status` (NEVER use the `-uall` flag).
- `git diff --stat`.

## Step 2: Classify

Group ALL changed files, including staged, unstaged, and untracked files, by path prefix:

| Path prefix | Category | Commit prefix |
|-------------|----------|---------------|
| `roam/main/` | Atomic notes | `note:` |
| `roam/reference/` | Reference notes | `ref:` |
| `roam/daily/` | Daily notes | `daily:` |
| `roam/projects/` | Project notes | `proj:` |
| `roam/read_history/` | Reading history | `read:` |
| `roam/toolkit/` | Toolkit collection | `toolkit:` |
| `.claude/` | Configuration | `config:` |
| Other | General | `feat:` / `fix:` / `refactor:` / `chore:` |

**Prefix selection:**
- ALL files in one category: use that category's prefix.
- Multiple categories: use a conventional commits prefix.

For `.org` files, extract the note title from the filename by stripping the timestamp prefix and replacing `_` with spaces.

## Step 3: Generate commit message

**Subject line:** `<prefix> <concise English summary>`, max 72 chars, lowercase after prefix.

**Body:** only include a bullet list of key changes when more than 3 files changed.

## Step 4: Execute

1. `git add -A` to stage everything.
2. `git commit` with HEREDOC:
   ```bash
   git commit -m "$(cat <<'EOF'
   <subject>

   <body>
   EOF
   )"
   ```
3. `git push` to remote.
4. `git status` to verify.

## Hard constraints

- Commit message in English.
- NEVER add attribution: no `Co-Authored-By`, no `Generated with`, nothing.
