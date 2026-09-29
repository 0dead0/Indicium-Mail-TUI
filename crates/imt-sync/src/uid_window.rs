//! UID range policy for envelope sync (recent window on first pass).

use imt_net::backend::UidRange;

/// How many recent UIDs to fetch on first sync / attachment scan / uidvalidity resync.
/// Aligned with the TUI hydrate cap (`snapshot` loads 500 messages per folder).
pub const INITIAL_SYNC_UID_WINDOW: u32 = 500;

/// Inclusive UID range for the most recent `window` messages before `server_uid_next`.
///
/// Returns `None` when the folder has no messages (`server_uid_next <= 1`).
pub fn recent_uid_window(server_uid_next: u32, window: u32) -> Option<(u32, u32)> {
    if server_uid_next <= 1 || window == 0 {
        return None;
    }
    let end = server_uid_next - 1;
    let start = end.saturating_sub(window.saturating_sub(1)).max(1);
    Some((start, end))
}

/// Decide which UID range to fetch for a folder sync pass.
///
/// - First sync / uidvalidity resync → recent window (not full history).
/// - Attachment scan, caught up → recent window for rescan.
/// - Attachment scan, behind cursor → from `min(last_uid_next, window_start)` through tip
///   (never skip the incremental cursor).
/// - Incremental → `(last_uid_next .. server_uid_next - 1)` when there is new mail.
/// - Otherwise → `None` (nothing to fetch).
pub fn envelope_sync_range(
    last_uid_next: u32,
    server_uid_next: u32,
    needs_full_resync: bool,
    need_attachment_scan: bool,
    window: u32,
) -> Option<UidRange> {
    if needs_full_resync || last_uid_next == 0 {
        return recent_uid_window(server_uid_next, window).map(|(s, e)| UidRange::Range(s, e));
    }
    if need_attachment_scan {
        let end = server_uid_next.saturating_sub(1);
        if end == 0 {
            return None;
        }
        let start = if server_uid_next > last_uid_next {
            // Must not skip the incremental cursor; also rescan the recent window.
            match recent_uid_window(server_uid_next, window) {
                Some((window_start, _)) => last_uid_next.min(window_start),
                None => last_uid_next,
            }
        } else {
            recent_uid_window(server_uid_next, window)?.0
        };
        return Some(UidRange::Range(start, end));
    }
    if server_uid_next > last_uid_next {
        let end = server_uid_next.saturating_sub(1).max(last_uid_next);
        Some(UidRange::Range(last_uid_next, end))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_sync_uses_recent_window_not_full_history() {
        // Given: never synced, server has many UIDs
        let server_uid_next = 10_001;
        let window = 500;

        // When: recent window is computed
        let range = recent_uid_window(server_uid_next, window);

        // Then: only the last N UIDs (not 1..10000)
        assert_eq!(range, Some((9_501, 10_000)));
    }

    #[test]
    fn recent_window_clamps_to_uid_one() {
        // Given: fewer messages than the window
        // When / Then:
        assert_eq!(recent_uid_window(10, 500), Some((1, 9)));
    }

    #[test]
    fn empty_folder_has_no_recent_window() {
        // Given: no messages
        // When / Then:
        assert_eq!(recent_uid_window(1, 500), None);
        assert_eq!(recent_uid_window(0, 500), None);
    }

    #[test]
    fn envelope_range_first_pass_is_window() {
        // Given: local uid_next 0 (never synced)
        // When: sync range is chosen
        let range = envelope_sync_range(0, 10_001, false, false, 500);

        // Then: recent window, not 1..10000
        assert_eq!(range, Some(UidRange::Range(9_501, 10_000)));
    }

    #[test]
    fn envelope_range_incremental_after_window() {
        // Given: already synced through server uid_next 1000, new mail arrived
        // When: sync range is chosen
        let range = envelope_sync_range(1_000, 1_005, false, false, 500);

        // Then: only new UIDs
        assert_eq!(range, Some(UidRange::Range(1_000, 1_004)));
    }

    #[test]
    fn envelope_range_caught_up_is_none() {
        // Given: local and server uid_next match
        // When / Then:
        assert_eq!(envelope_sync_range(100, 100, false, false, 500), None);
    }

    #[test]
    fn attachment_scan_uses_window_not_full_history() {
        // Given: folder already has uid_next but attachment scan pending
        // When: sync range is chosen
        let range = envelope_sync_range(10_001, 10_001, false, true, 500);

        // Then: recent window only
        assert_eq!(range, Some(UidRange::Range(9_501, 10_000)));
    }

    #[test]
    fn attachment_scan_does_not_skip_incremental_cursor() {
        // Given: folder synced through uid 1000, many new messages, attachment scan pending
        let last_uid_next = 1_000;
        let server_uid_next = 10_001;

        // When: sync range is chosen
        let range = envelope_sync_range(last_uid_next, server_uid_next, false, true, 500);

        // Then: fetch from cursor through tip (not window-only, which would drop 1000..9500)
        assert_eq!(range, Some(UidRange::Range(1_000, 10_000)));
    }

    #[test]
    fn first_pass_empty_folder_fetches_nothing() {
        // Given: never synced, server reports empty mailbox
        // When / Then: no UID range (caller still persists uid_next)
        assert_eq!(envelope_sync_range(0, 1, false, false, 500), None);
    }

    #[test]
    fn uidvalidity_resync_uses_window() {
        // Given: uidvalidity changed (full resync flag)
        // When / Then: recent window, not full history
        assert_eq!(
            envelope_sync_range(10_001, 10_001, true, false, 500),
            Some(UidRange::Range(9_501, 10_000))
        );
    }
}
