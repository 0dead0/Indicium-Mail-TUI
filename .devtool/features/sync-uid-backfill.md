---
id: "sync-uid-backfill"
status: "ops"
priority: "medium"
assignee: null
epic: null
dueDate: null
created: "2026-09-29T11:24:00.000Z"
modified: "2026-09-29T12:28:00.601Z"
completedAt: null
labels: ["sync", "gmail", "performance"]
order: "Zz"
---
# Background UID backfill (older mail after recent window)

## Story

As a user on a large mailbox  
I want older messages to fill in gradually after the first recent-UID sync  
so that Inbox stays fast on connect while history still becomes searchable/readable over time.

## Context

- Follow-up to `sync-recent-uid-window` (v1): first pass fetches only last N UIDs (`INITIAL_SYNC_UID_WINDOW=500`); **backfill was deferred**.
- Compatible with `sync-inbox-first-on-demand` and `skip-noselect-folders`.
- `uid_next` must stay the **forward** cursor for incremental new mail. Backfill must not reset it or reintroduce `1..uid_next-1` full fetches.

## Product choices

- **Separate backward cursor** `backfill_low` in table `folder_backfill`: lowest UID already fetched; complete when `<= 1`.
- **Chunk size:** same N as `INITIAL_SYNC_UID_WINDOW` (500).
- **Scheduling:** one chunk after ~45s quiet IDLE; never before Inbox sync + IDLE.
- **Priority:** IDLE/current folder → Inbox → others; **Gmail All Mail skipped by default**.
- **Existing full DBs:** migration `0007` marks `MIN(uid) <= 1` complete; partial history seeds at `MIN(uid)`.
- **UI:** `SyncStarted` / `SyncFinished` on each chunk (status spinner).

## Acceptance criteria

- Given a folder after first window sync (`uid_next` at server tip, history incomplete), When backfill runs, Then it fetches the next older chunk `Range(max(1, backfill_low - N) .. backfill_low - 1)` and lowers `backfill_low`.
- Given backfill runs, When new mail arrives, Then incremental sync via `uid_next` still works and is not blocked by backfill.
- Given connect, When the account worker starts, Then backfill does not run before Inbox sync + IDLE.
- Given on-demand open / SyncFolder for new mail, When both are pending, Then they take priority over backfill.
- Given `\Noselect` / bare `[Gmail]`, When backfill selects work, Then those paths are never selected.
- Given a folder already fully present locally, When migration/backfill init runs, Then it is marked complete without a historical IMAP dump.
- Given tests, When run, Then Given/When/Then coverage for cursor math, priority vs incremental, and “already complete” without live Gmail.

## Implementation notes

- `imt-sync/src/uid_window.rs`: `BackfillChunk`, `backfill_chunk`, `seed_backfill_low`.
- `imt-sync/src/backfill.rs`: phase gate, All Mail skip, `pick_backfill_folder`, `compute_backfill_seed` / `seed_backfill_cursor_if_needed`.
- `account_task`: seed after envelope sync; `run_one_backfill_chunk` on IDLE quiet timer; backfill `MessageAdded { notify: false }`.
- `engine::sync_folder`: same seed; live sync `notify: true`.
- Store: migration `0007_folder_backfill.sql`; `FolderRepo::backfill_low` / `set_backfill_low`; `MessageRepo::min_uid`.
- Docs: `DOCUMENTATION.md` sync flow; `CHANGELOG.md` Unreleased.

## Review fixes (Bugbot)

- Backfill emits `MessageAdded { notify: false }` so historical unread does not toast.
- UIDVALIDITY (`needs_full_resync`) always reseeds `backfill_low` via `compute_backfill_seed`.
- Missing local messages (`min_uid = None`) seeds from the recent window instead of marking complete (`1`).

## Next

Human re-review of Bugbot fixes → then ops install/smoke when accepted.

## Tests (green)

Name: backfill_chunk_fetches_next_older_window  
Given: folder with uid_next at tip and backfill_low = 9501, N = 500  
When: next backfill range is computed  
Then: Range(9001, 9500) and backfill_low becomes 9001 after the chunk

Name: backfill_final_chunk_clamps_to_uid_one  
Given: backfill_low = 200, N = 500  
When: next backfill range is computed  
Then: Range(1, 199) and afterward backfill is complete (backfill_low <= 1)

Name: backfill_complete_requests_nothing  
Given: backfill_low <= 1  
When: next backfill range is computed  
Then: no IMAP fetch range

Name: first_window_sync_seeds_backfill_low  
Given: never-synced folder, server uid_next = 10001, window = 500  
When: first envelope sync finishes  
Then: uid_next = 10001 and backfill_low = 9501

Name: incremental_new_mail_ignores_backfill_cursor  
Given: uid_next = 10001, backfill_low = 9001, server tip advanced to 10005  
When: normal envelope sync range is chosen  
Then: Range(10001, 10004) only (uid_next unchanged by backfill)

Name: connect_reaches_idle_before_any_backfill  
Given: account with inbox plus other incomplete folders  
When: connect sync runs  
Then: inbox sync then IDLE happen with zero backfill fetches beforehand

Name: on_demand_beats_backfill  
Given: stale folder open requested and a backfill chunk also pending  
When: the worker picks next work  
Then: on-demand SyncFolder runs first

Name: noselect_never_chosen_for_backfill  
Given: leftover [Gmail] row and incomplete real folders  
When: backfill picks a folder  
Then: [Gmail] is not selected

Name: already_full_local_history_skips_imap_dump  
Given: folder whose local MIN(uid) <= 1 (or migration marks complete)  
When: backfill init / migration runs  
Then: backfill_low = 1 and no historical UID FETCH

Name: uidvalidity_resync_reseeds_existing_cursor  
Given: folder_backfill already has a stale cursor  
When: UIDVALIDITY changes  
Then: backfill_low recomputed from the new recent window

Name: empty_local_store_does_not_mark_backfill_complete  
Given: uid_next set, min_uid = None  
When: backfill cursor seeded  
Then: recent-window seed (not backfill_low = 1)

Name: backfill_message_added_does_not_request_toast  
Given: historical envelopes from a backfill chunk  
When: MessageAdded emitted  
Then: notify = false

## Out of scope

- Changing first-pass window size or inbox-first scheduling.
- SyncAll = current-folder-only (separate if needed).
- SQLite batch upserts / multi-process single-writer.
- Full attachment rescan of historical mail (backfill may set `has_attachments` from BODYSTRUCTURE as chunks arrive; no forced full-folder scan).