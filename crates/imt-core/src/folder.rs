use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AccountId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FolderId(pub Uuid);

impl FolderId {
    pub fn new() -> Self { Self(Uuid::new_v4()) }
}

impl Default for FolderId {
    fn default() -> Self { Self::new() }
}

/// Special-use semantic role of a folder (RFC 6154).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FolderRole {
    Inbox,
    Sent,
    Drafts,
    Trash,
    Junk,
    Archive,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    pub id: FolderId,
    pub account_id: AccountId,
    /// Server path, e.g. `INBOX`, `[Gmail]/Sent Mail`.
    pub path: String,
    /// Display name (last path segment).
    pub name: String,
    pub role: FolderRole,
    pub uid_validity: u32,
    pub uid_next: u32,
    pub message_count: u32,
    pub unread_count: u32,
}

impl Folder {
    /// True when this folder has never completed an envelope sync locally.
    ///
    /// Used to decide whether opening a folder should trigger an on-demand sync.
    /// `uid_validity` mismatches are detected later inside a real SELECT/sync.
    pub fn is_stale(&self) -> bool {
        self.uid_next == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AccountId;

    fn folder(uid_next: u32) -> Folder {
        Folder {
            id: FolderId::new(),
            account_id: AccountId::new(),
            path: "Sent".into(),
            name: "Sent".into(),
            role: FolderRole::Sent,
            uid_validity: 1,
            uid_next,
            message_count: 0,
            unread_count: 0,
        }
    }

    #[test]
    fn never_synced_folder_is_stale() {
        // Given: a folder that has never been envelope-synced
        let f = folder(0);

        // When: stale is checked
        let stale = f.is_stale();

        // Then: it is stale
        assert!(stale);
    }

    #[test]
    fn synced_folder_is_not_stale() {
        // Given: a folder that has completed at least one envelope sync
        let f = folder(42);

        // When: stale is checked
        let stale = f.is_stale();

        // Then: it is not stale
        assert!(!stale);
    }
}
