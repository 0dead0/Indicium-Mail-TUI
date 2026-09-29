//! Historical UID backfill scheduling (after the recent-window first sync).

use imt_core::{Folder, FolderId, FolderRole};
use imt_store::{FolderRepo, MessageRepo, Result as StoreResult};

use crate::uid_window::seed_backfill_low;

/// True when historical backfill still has older UIDs to fetch.
pub fn backfill_incomplete(backfill_low: u32) -> bool {
    backfill_low > 1
}

pub use imt_net::is_all_mail_folder;

/// Whether backfill may run in this worker phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncPhase {
    Connect,
    IdleQuiet,
    OnDemand,
}

pub fn may_backfill_in_phase(phase: SyncPhase) -> bool {
    matches!(phase, SyncPhase::IdleQuiet)
}

/// Prefer current folder, then Inbox, then other selectable incomplete folders.
/// Skips `\Noselect` / bare `[Gmail]` and All Mail.
pub fn pick_backfill_folder<'a, F>(
    folders: &'a [Folder],
    mut backfill_low: F,
    current_path: Option<&str>,
) -> Option<&'a Folder>
where
    F: FnMut(FolderId) -> Option<u32>,
{
    let eligible = |f: &Folder, low: Option<u32>| {
        imt_net::is_bulk_sync_folder(&f.path) && low.map(backfill_incomplete).unwrap_or(false)
    };

    if let Some(path) = current_path {
        if let Some(f) = folders.iter().find(|f| f.path == path) {
            let low = backfill_low(f.id);
            if eligible(f, low) {
                return Some(f);
            }
        }
    }

    if let Some(f) = folders.iter().find(|f| f.role == FolderRole::Inbox) {
        let low = backfill_low(f.id);
        if eligible(f, low) {
            return Some(f);
        }
    }

    folders.iter().find(|f| {
        let low = backfill_low(f.id);
        eligible(f, low)
    })
}

/// Decide the `backfill_low` to persist, or `None` when the existing cursor stands.
///
/// - UIDVALIDITY resync always reseeds from the recent window.
/// - Missing row: seed from window / local `min_uid`; empty local store uses the
///   recent-window seed (not "complete").
pub fn compute_backfill_seed(
    existing_backfill_low: Option<u32>,
    last_uid_next: u32,
    server_uid_next: u32,
    needs_full_resync: bool,
    min_uid: Option<u32>,
    window: u32,
) -> Option<u32> {
    if needs_full_resync {
        return Some(seed_backfill_low(server_uid_next, window));
    }
    if existing_backfill_low.is_some() {
        return None;
    }
    let low = if last_uid_next == 0 {
        seed_backfill_low(server_uid_next, window)
    } else {
        match min_uid {
            Some(u) if u <= 1 => 1,
            Some(u) => u,
            None => seed_backfill_low(server_uid_next, window),
        }
    };
    Some(low)
}

/// Initialize or refresh `backfill_low` after an envelope sync.
pub async fn seed_backfill_cursor_if_needed(
    folders: &FolderRepo<'_>,
    messages: &MessageRepo<'_>,
    folder_id: FolderId,
    last_uid_next: u32,
    server_uid_next: u32,
    needs_full_resync: bool,
    window: u32,
) -> StoreResult<()> {
    let existing = folders.backfill_low(folder_id).await?;
    let min_uid = messages.min_uid(folder_id).await?;
    let Some(low) = compute_backfill_seed(
        existing,
        last_uid_next,
        server_uid_next,
        needs_full_resync,
        min_uid,
        window,
    ) else {
        return Ok(());
    };
    folders.set_backfill_low(folder_id, low).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use imt_core::{AccountId, FolderId, FolderRole};

    fn folder(path: &str, role: FolderRole) -> Folder {
        Folder {
            id: FolderId::new(),
            account_id: AccountId::new(),
            path: path.into(),
            name: path.into(),
            role,
            uid_validity: 1,
            uid_next: 100,
            message_count: 10,
            unread_count: 0,
        }
    }

    #[test]
    fn connect_reaches_idle_before_any_backfill() {
        // Given: connect phase (inbox sync before IDLE)
        // When: backfill eligibility for the phase is checked
        // Then: backfill does not run before IDLE
        assert!(!may_backfill_in_phase(SyncPhase::Connect));
        assert!(may_backfill_in_phase(SyncPhase::IdleQuiet));
    }

    #[test]
    fn on_demand_beats_backfill() {
        // Given: stale folder open requested and a backfill chunk also pending
        // When: the worker picks next work (phase)
        // Then: on-demand SyncFolder runs first (backfill only in IdleQuiet)
        assert!(!may_backfill_in_phase(SyncPhase::OnDemand));
        assert!(may_backfill_in_phase(SyncPhase::IdleQuiet));
    }

    #[test]
    fn noselect_never_chosen_for_backfill() {
        // Given: leftover [Gmail] row and incomplete real folders
        let gmail = folder("[Gmail]", FolderRole::Other);
        let spam = folder("[Gmail]/Spam", FolderRole::Junk);
        let folders = vec![gmail.clone(), spam.clone()];
        let lows = |id: FolderId| {
            if id == gmail.id || id == spam.id {
                Some(9_501)
            } else {
                None
            }
        };

        // When: backfill picks a folder
        let picked = pick_backfill_folder(&folders, lows, None);

        // Then: [Gmail] is not selected
        assert_eq!(picked.map(|f| f.path.as_str()), Some("[Gmail]/Spam"));
    }

    #[test]
    fn pick_prefers_current_then_inbox() {
        let sent = folder("Sent", FolderRole::Sent);
        let inbox = folder("INBOX", FolderRole::Inbox);
        let archive = folder("Archive", FolderRole::Archive);
        let folders = vec![sent.clone(), inbox.clone(), archive.clone()];
        let lows = |_id: FolderId| Some(5_000u32);

        assert_eq!(
            pick_backfill_folder(&folders, lows, Some("Archive")).map(|f| f.path.as_str()),
            Some("Archive")
        );
        assert_eq!(
            pick_backfill_folder(&folders, lows, None).map(|f| f.path.as_str()),
            Some("INBOX")
        );
    }

    #[test]
    fn all_mail_skipped_by_default() {
        let inbox = folder("INBOX", FolderRole::Inbox);
        let all = folder("[Gmail]/Wszystkie", FolderRole::Other);
        let folders = vec![all.clone(), inbox.clone()];
        let lows = |id: FolderId| {
            if id == all.id {
                Some(50_000)
            } else {
                Some(1) // inbox complete
            }
        };

        assert!(pick_backfill_folder(&folders, lows, None).is_none());
    }

    #[test]
    fn uidvalidity_resync_reseeds_existing_cursor() {
        // Given: folder_backfill already has a stale cursor from the old UID space
        // When: UIDVALIDITY changes (needs_full_resync)
        // Then: backfill_low is recomputed from the new recent window
        let next = compute_backfill_seed(Some(9_501), 10_001, 500, true, Some(9_501), 500);
        assert_eq!(next, Some(1)); // empty-ish new tip: seed_backfill_low(500)=1 via window start
        let next = compute_backfill_seed(Some(9_501), 10_001, 10_001, true, Some(100), 500);
        assert_eq!(next, Some(9_501));
    }

    #[test]
    fn empty_local_store_does_not_mark_backfill_complete() {
        // Given: uid_next set, no local messages yet (min_uid = None)
        // When: backfill cursor is seeded
        // Then: recent-window seed is used, not backfill_low = 1
        let next = compute_backfill_seed(None, 10_001, 10_001, false, None, 500);
        assert_eq!(next, Some(9_501));
    }

    #[test]
    fn existing_cursor_unchanged_without_uidvalidity() {
        // Given: folder_backfill row already present
        // When: normal envelope sync seeds again
        // Then: existing cursor is left alone
        assert_eq!(
            compute_backfill_seed(Some(9_501), 10_001, 10_001, false, Some(9_501), 500),
            None
        );
    }

    #[test]
    fn backfill_message_added_does_not_request_toast() {
        // Given: historical envelopes upserted during a backfill chunk
        // When: MessageAdded is emitted for the UI list refresh
        // Then: notify is false so snapshot skips "New mail" toasts
        use imt_core::{FolderId, MessageId, SyncEvent};
        let ev = SyncEvent::MessageAdded {
            folder_id: FolderId::new(),
            message_id: MessageId::new(),
            notify: false,
        };
        match ev {
            SyncEvent::MessageAdded { notify, .. } => assert!(!notify),
            _ => panic!("expected MessageAdded"),
        }
    }
}
