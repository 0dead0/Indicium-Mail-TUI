---
name: kanban
description: >
  Lists and manages Kanban Markdown board cards under `.devtool/features/`.
  Use when the user runs /kanban, /kanban list, or asks to list kanban tasks/cards/statuses.
disable-model-invocation: true
---

# Kanban

Operate on [Kanban Markdown](https://marketplace.visualstudio.com/items?itemName=LachyFS.kanban-markdown) cards: markdown files with YAML frontmatter under the workspace features directory.

## Board location

1. Prefer `kanban-markdown.featuresDirectory` from workspace/user settings if set.
2. Default: `.devtool/features/` (relative to workspace root).
3. Cards = `**/*.md` recursively (status subfolders optional; also flat files).

## Frontmatter (per card)

| Field | Use |
|-------|-----|
| `id` | Stable card id |
| `status` | Column: `backlog`, `todo`, `in-progress`, `review`, `ops`, `done` (workspace columns in `.vscode/settings.json`) |
| `priority` | Optional |
| `order` | Optional sort within column |

Title = first `#` heading in body (fallback: filename stem).

## Commands

### `/kanban list`

List all cards and statuses.

1. Resolve features directory (above).
2. If missing / empty → say so; stop.
3. For each `.md`: read frontmatter `status` + title.
4. Group by status. Column order: `backlog` → `todo` → `in-progress` → `review` → `ops` → `done`, then any unknown statuses alphabetically.
5. Within a column: sort by `order` if present, else title.
6. Output compact (simple mode OK):

```
backlog (N)
- Title
todo (N)
- …
```

Include count per column. Skip empty columns unless board has zero cards total.

Do **not** dump full card bodies on `list`.

## Future commands

Only `list` is defined. Unknown `/kanban …` → say supported: `list`.
