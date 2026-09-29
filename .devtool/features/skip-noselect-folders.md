---
id: "skip-noselect-folders"
status: "ops"
priority: "medium"
assignee: null
epic: null
dueDate: null
created: "2026-09-29T08:31:00.000Z"
modified: "2026-09-29T11:10:32.772Z"
completedAt: null
labels: ["gmail", "sync", "imap"]
order: "a2"
---
# Sync skips non-selectable IMAP folders (e.g. Gmail `[Gmail]`)

## Story

As a Gmail user  
I want the client not to `SELECT` IMAP `\Noselect` parents (like bare `[Gmail]`)  
so that every connect does not waste a failing round-trip, emit bogus sync errors, or add pressure when Gmail is already `THROTTLED`.

## Context

- Gmail LIST returns `[Gmail]` as a hierarchy prefix; it is not a real mailbox (`\Noselect` / `NONEXISTENT` on SELECT).
- `list_folders` still runs STATUS on every LIST name (including `[Gmail]`). SyncAll / on-demand can SELECT stored folders.
- After inbox-first, connect no longer envelope-syncs every folder at startup; STATUS-on-LIST and SyncAll remain the live failure paths.
- Log: `folder sync [Gmail]: select [Gmail]: ... [NONEXISTENT] Unknown Mailbox: [Gmail] ...` often with `[THROTTLED]`.
- Real slowdown for huge mailboxes is separate; this card is only the `\Noselect` / fake-parent fix.

## Acceptance criteria

- Given an IMAP LIST entry marked `\Noselect` (or equivalent), When folder list is built / sync runs, Then that path is never `SELECT`ed or envelope-synced.
- Given Gmail’s bare `[Gmail]` parent, When the account connects, Then no WARN/sync error for selecting `[Gmail]`.
- Given selectable children (`[Gmail]/Spam`, All Mail, etc.), When sync runs, Then they still sync as today.
- Given the sidebar, When folders are shown, Then `\Noselect` parents are **omitted** (not shown as tree nodes).

## Product choice

- **UI:** omit `\Noselect` / bare `[Gmail]` from persisted folder list and sidebar (not shown as non-syncable nodes).

## Suggested approach

- Detect `\Noselect` from IMAP LIST attributes; hard-skip bare `[Gmail]` if attributes are missing.
- Skip STATUS / SELECT / envelope sync for non-selectable paths; omit them from `list_folders` output.
- Guard `sync_one_folder` / `SyncFolder` so leftover DB rows for `[Gmail]` are not selected.
- Tests: Given/When/Then unit coverage; no live Gmail required.

## Implementation notes

- `is_selectable_mailbox(path, listed_noselect)` in `imt-net` — `\Noselect` and bare `[Gmail]` hard-skip.
- `list_folders` skips STATUS and omits non-selectable names (sidebar never sees them).
- `sync_one_folder` / `SyncEngine::sync_folder` skip leftover DB rows for non-selectable paths.
- UI choice: omit from sidebar (not shown as tree nodes).

## Out of scope

- Cap first-sync UID range / defer All Mail.
- Batch SQLite upserts / multi-process DB contention.
- General Gmail throttle backoff policy.
- Pruning stale `[Gmail]` rows already in SQLite (skipped on sync; not deleted).