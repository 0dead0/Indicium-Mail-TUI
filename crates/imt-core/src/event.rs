use serde::{Deserialize, Serialize};

use crate::{AccountId, Flag, FolderId, MessageId, Uid};

fn default_notify() -> bool {
    true
}

/// Events emitted by the sync engine, consumed by the TUI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncEvent {
    AccountConnecting { account_id: AccountId },
    AccountConnected { account_id: AccountId },
    AccountDisconnected { account_id: AccountId, reason: String },

    FolderListUpdated { account_id: AccountId },
    FolderCountsChanged { folder_id: FolderId, total: u32, unread: u32 },

    /// Envelope upserted locally. `notify` is false for historical backfill so the
    /// UI refreshes without "New mail" toasts.
    MessageAdded {
        folder_id: FolderId,
        message_id: MessageId,
        #[serde(default = "default_notify")]
        notify: bool,
    },
    MessageRemoved { folder_id: FolderId, uid: Uid },
    MessageFlagsChanged { message_id: MessageId, flags: Vec<Flag> },
    MessageBodyFetched { message_id: MessageId },

    SyncStarted { account_id: AccountId, folder_id: Option<FolderId> },
    SyncFinished { account_id: AccountId, folder_id: Option<FolderId> },

    Error { account_id: Option<AccountId>, message: String },
}
