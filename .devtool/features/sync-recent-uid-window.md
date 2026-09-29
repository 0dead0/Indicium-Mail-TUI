---
id: "sync-recent-uid-window"
status: "ops"
priority: "high"
assignee: null
epic: null
dueDate: null
created: "2026-09-29T08:35:00.000Z"
modified: "2026-09-29T11:10:35.779Z"
completedAt: null
labels: ["sync", "gmail", "performance"]
order: "a3"
---
# First sync uses a recent UID window (then backfill)

## Story

As a user with a huge folder (e.g. Gmail Inbox / All Mail)  
I want the first envelope fetch to cover only recent messages (last N UIDs)  
so that initial sync finishes in seconds/minutes, not a full historical dump of 80k–130k messages.

## Context

- First sync and one-time attachment scan use `UidRange::Range(1, uid_next-1)` / `All` when `last_uid_next == 0` or `need_attachment_scan`.
- UI hydrate already caps at 500 messages per folder (`snapshot.rs`).
- Related: inbox-first / on-demand (when folders sync); this card is **how much** is fetched on first pass.

## Product choices (v1 = option A)

- **Scope:** windowed first fetch only; **backfill deferred** to a follow-up card.
- **N default:** `500` (aligned with UI hydrate cap). Constant `INITIAL_SYNC_UID_WINDOW`.
- **Attachment scan / uidvalidity resync:** same recent window — **not** full-history (documented departure from “scan everything”).
- **Backfill cursor:** not a new DB column in v1; follow-up may use `MIN(uid)` of stored messages or add `lowest_synced_uid`.

## Acceptance criteria (v1)

- Given a folder never synced (`uid_next` local 0), When envelopes are fetched, Then only the last N UIDs are requested, not `1..uid_next-1`.
- Given that window is persisted, When sync finishes the first pass, Then local `uid_next` allows normal incremental sync for new mail.
- Given attachment scan / full resync triggers, When envelopes are fetched, Then the same window policy applies (not full history).
- Given tests, When run, Then Given/When/Then coverage exists for window vs incremental without live Gmail.

## Out of scope (v1)

- Background / IDLE backfill of older UIDs (follow-up).
- Inbox-first / on-demand / `\Noselect` (other cards).
- Disabling Gmail All Mail auto-sync.
- SQLite batching.

## Suggested approach

- Shared helper: recent window `(uid_next - N) .. (uid_next - 1)` clamped to ≥ 1.
- Use from `account_task` and `engine.sync_folder`.

## Implementation notes (v1)

- `imt-sync/src/uid_window.rs`: `INITIAL_SYNC_UID_WINDOW=500`, `recent_uid_window`, `envelope_sync_range`.
- Wired in `account_task::sync_one_folder` and `SyncEngine::sync_folder`.
- Empty / caught-up: no fetch, but SELECT state (`uid_next`, counts) still persisted.
- Backfill: not implemented (follow-up card).