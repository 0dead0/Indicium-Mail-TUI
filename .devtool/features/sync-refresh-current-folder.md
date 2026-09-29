---
id: "sync-refresh-current-folder"
status: "ops"
priority: "high"
assignee: null
epic: null
dueDate: null
created: "2026-09-29T14:08:00.000Z"
modified: "2026-09-29T14:30:58.586Z"
completedAt: null
labels: ["sync", "gmail", "performance"]
order: "Zy"
---
# Auto-refresh / SyncAll use current folder (skip All Mail bulk)

## Story

As a Gmail user  
I want periodic refresh not to walk every folder (especially All Mail)  
so that the client does not freeze or get THROTTLED every auto-refresh interval.

## Acceptance criteria

- Given auto-refresh fires, When a folder is selected, Then only that folder is synced.
- Given SyncAccount / SyncAll, When folders include All Mail or bare `[Gmail]`, Then those are skipped in the bulk poll (open All Mail still syncs on demand via SyncFolder).
- Given LIST omits leftover `[Gmail]` DB rows, When folder list syncs, Then those rows are pruned from the DB and sidebar.
- Given attachment-scan comments / docs, When read, Then they describe the recent-UID window (not “full rescan”).

## Implementation notes

- TUI `tick` auto-refresh → `refresh(Some(acc), Some(folder))`.
- `imt_net::is_all_mail_folder` / `is_bulk_sync_folder` / `should_retain_stored_folder`.
- `command_worker` SyncAccount/SyncAll filter with `is_bulk_sync_folder`.
- `sync_folder_list` deletes non-retained stored folders; `FolderRepo::delete` clears side tables.
- Docs: CHANGELOG, DOCUMENTATION, README, settings hint.
- Board: removed duplicate backlog copies + duplicate `message-list-scroll-1`.

## Departure from `sync-inbox-first-on-demand`

That card said “Auto-refresh always SyncAll”. **Departed:** auto-refresh is current-folder-only; SyncAll skips All Mail / non-selectable.

## Tests

- `auto_refresh_requests_current_folder_only` (imt-tui)
- `all_mail_is_excluded_from_bulk_sync`, `leftover_gmail_parent_is_not_retained_after_list` (imt-net)