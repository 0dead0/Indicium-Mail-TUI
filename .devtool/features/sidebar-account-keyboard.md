---
id: "sidebar-account-keyboard"
status: "in-progress"
priority: "medium"
assignee: null
epic: null
dueDate: null
created: "2026-09-27T18:19:00.000Z"
modified: "2026-09-27T18:30:00.000Z"
labels: []
order: "a1"
---
# User can expand accounts and move panes with keyboard

## Story

As a multi-account user  
I want account rows in the sidebar to be selectable and expandable with Enter / ←→  
so that I can unfold folders and move between panes without the mouse.

## Acceptance criteria

- Given focus on a collapsed account header, When Enter or →, Then the account expands.
- Given focus on an expanded account header, When Enter or ←, Then the account collapses.
- Given focus on a folder, When Enter or →, Then focus moves to the message list.
- Given focus on a folder, When ←, Then selection moves to the parent account header.
- Given focus on list/reader, When ←/→, Then focus moves Sidebar ↔ List ↔ Reader.
- Given collapsed accounts, When j/k in the sidebar, Then only visible rows (headers + expanded folders) are visited.

## Approach

- `sidebar_on_account` cursor flag; highlight account header when set.
- Enter / OpenMessage: toggle expand on account, else focus list from folder.
- Normal-mode Left/Right (`PaneLeft` / `PaneRight`): expand/collapse/parent + directional pane focus.
- Sidebar j/k walks visible tree only.
- Shared `toggle_account_expanded()` for Enter and mouse click.

## Tests

Unit tests in `app::sidebar_nav_tests` (9 cases). `cargo test -p imt-tui --lib` green (17).

## Docs

README keys + mouse, help overlay, status bar, CHANGELOG Unreleased, DOCUMENTATION mouse/sidebar note.

## Next

Ready for human review → ask to move to `review`.
