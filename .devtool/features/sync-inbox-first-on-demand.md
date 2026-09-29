---
id: "sync-inbox-first-on-demand"
status: "ops"
priority: "high"
assignee: null
epic: null
dueDate: null
created: "2026-09-29T08:35:00.000Z"
modified: "2026-09-29T11:10:31.252Z"
completedAt: null
labels: ["sync", "gmail", "performance"]
order: "a1"
---
# Sync inbox first; other folders on demand

## Story

As a user with a large mailbox  
I want connect to sync the inbox (then IDLE) before bulk-syncing every folder  
so that the client becomes usable quickly and does not look frozen on Gmail-scale accounts.

## Context

- Today `account_task` lists folders then `sync_one_folder` for **every** folder before entering IDLE (`crates/imt-sync/src/account_task.rs`).
- Gmail can have tens of thousands of messages across many labels; sequential full sync blocks “ready” for a long time.
- Related: `skip-noselect-folders` (don’t SELECT `\Noselect`); `sync-recent-uid-window` (cap first-fetch size). This card is **scheduling**, not UID range.

## Acceptance criteria

- Given account connect, When the worker starts, Then Inbox (role `inbox`, else first real folder) is synced before other folders.
- Given Inbox sync finished, When no other work is required for interactivity, Then the worker enters IDLE on Inbox without waiting for all folders.
- Given the user opens a non-inbox folder that is **stale**, When that happens, Then that folder is synced on demand.
- Given the user opens a folder that is **not** stale, When that happens, Then no sync is triggered from open.
- Given folders not yet visited, When viewing the sidebar, Then they still appear from LIST/STATUS (or cached metadata) without implying a completed envelope sync.
- Given auto-refresh / SyncAll, When it fires, Then **current-folder / bulk-safe** sync runs (see `sync-refresh-current-folder` — **departure:** no longer full SyncAll of every folder).

## Product choices (recorded)

- **Primary at connect:** Inbox only (role `Inbox`, else first folder). **Sent stays on-demand** — not primary.
- **Open → sync only when stale.** Stale = local `uid_next == 0` (never envelope-synced). `uid_validity` mismatch is handled inside an actual sync (`sync_one_folder` / SyncFolder / SyncAll), not as an open-time predicate without SELECT.
- **Auto-refresh always SyncAll** — **departed** → current folder only (`sync-refresh-current-folder`).

## Suggested approach

- Split startup: sync primary folder → IDLE; defer the rest.
- On folder open: if stale, call `refresh(account, folder)` → `SyncFolder` (second connection OK for occasional opens).
- Ctrl-R refreshes current folder/account; SyncAll skips All Mail (see `sync-refresh-current-folder`).
- Preserve incremental `uid_next` behavior once a folder has been synced once.
- Status line should show which folder is syncing when deferred work runs.

## Implementation notes

- `primary_folder_for_sync` + connect loop in `account_task.rs` (Inbox only → IDLE).
- `Folder::is_stale()` (`uid_next == 0`) in `imt-core`.
- TUI `open_selected_folder` on sidebar/mouse/account cycle; refresh narrowed in `sync-refresh-current-folder`.
- Tests: `imt-core` stale, `imt-sync` primary selection, `imt-tui` stale/fresh open.

## Out of scope

- Recent-UID window / backfill policy (`sync-recent-uid-window`).
- Skipping `\Noselect` / bare `[Gmail]` (`skip-noselect-folders`).
- Skipping Gmail All Mail by default (possible follow-up).
- SQLite batch upserts / multi-process writer.
- Single-connection interrupt queue for on-demand (option B) — revisit if Gmail throttle bites.