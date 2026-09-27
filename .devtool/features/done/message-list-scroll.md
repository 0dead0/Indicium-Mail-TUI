---
id: "message-list-scroll"
status: "done"
priority: "high"
assignee: null
epic: null
dueDate: null
created: "2026-09-27T12:58:29.953Z"
modified: "2026-09-27T14:20:00.000Z"
completedAt: "2026-09-27T14:20:00.000Z"
labels: []
order: "a0"
---
# Fix message list viewport scroll

## Symptom

`j`/`k` (and wheel) change selection but the list stays on the first screenful; rows below the viewport stay off-screen. Reader may still update.

## Cause

`crates/imt-tui/src/ui/list.rs` builds a Ratatui `Table` of all rows and styles `message_idx`, but does not use `TableState` / scroll offset. Paint always starts at row 0.

## Approach

- Mirror `accounts.rs`: `TableState` + `render_stateful_widget` + `highlight_style`.
- Persist or share scroll offset so `message_at_row` hit-testing stays correct (incl. 2-line snippets).
- Update `message_at_row` unit tests for scrolled viewport.

## Out of scope

- Raising the 500-message folder load cap in `snapshot.rs`.
- Infinite / lazy folder load.

## Tests (unit, same shape as `mouse_tests`)

Name: scrolled_list_first_visible_row_maps_to_offset_message  
Given: list pane with show_snippet off and list scroll offset past 0  
When: message_at_row is asked for the first inner row  
Then: it returns the message index at that offset (not always 0)

Name: scrolled_list_border_row_still_maps_to_nothing  
Given: list pane with a non-zero scroll offset  
When: message_at_row is asked for the top border row  
Then: it returns None

Name: scrolled_list_snippet_rows_use_height_two  
Given: show_snippet on, messages with snippets, and a non-zero list scroll offset  
When: message_at_row is asked for successive inner rows  
Then: indices advance by two terminal lines per message from the offset start

Not covered here: paint/viewport follow (needs manual `imt run --mock` or new render tests).

## Verify

- `cargo test -p imt-tui` (above cases) — done, 8 passed
- Manual: selection past viewport scrolls list; click after scroll selects the right row.
