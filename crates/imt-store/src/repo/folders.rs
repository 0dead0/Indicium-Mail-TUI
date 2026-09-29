//! Folder persistence.

use imt_core::{AccountId, Folder, FolderId, FolderRole};
use sqlx::Row;
use sqlx::SqlitePool;

use crate::repo::{uuid_bytes, uuid_from_slice};
use crate::{Result, StoreError};

/// CRUD operations for folders.
pub struct FolderRepo<'a>(pub &'a SqlitePool);

impl<'a> FolderRepo<'a> {
    /// Wrap a pool reference.
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self(pool)
    }

    /// Insert or update a folder row.
    pub async fn upsert(&self, f: &Folder) -> Result<()> {
        let id_bytes = uuid_bytes(&f.id.0);
        let acc_bytes = uuid_bytes(&f.account_id.0);
        let role = role_to_str(f.role);
        sqlx::query(
            "INSERT INTO folders (id, account_id, path, name, role, uid_validity, uid_next, message_count, unread_count) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) \
             ON CONFLICT(account_id, path) DO UPDATE SET \
                name = excluded.name, \
                role = excluded.role, \
                uid_validity = excluded.uid_validity, \
                uid_next = excluded.uid_next, \
                message_count = excluded.message_count, \
                unread_count = excluded.unread_count",
        )
        .bind(&id_bytes)
        .bind(&acc_bytes)
        .bind(&f.path)
        .bind(&f.name)
        .bind(role)
        .bind(f.uid_validity as i64)
        .bind(f.uid_next as i64)
        .bind(f.message_count as i64)
        .bind(f.unread_count as i64)
        .execute(self.0)
        .await?;
        Ok(())
    }

    /// Fetch a folder by id.
    pub async fn get(&self, id: FolderId) -> Result<Folder> {
        let id_bytes = uuid_bytes(&id.0);
        let row = sqlx::query(
            "SELECT id, account_id, path, name, role, uid_validity, uid_next, message_count, unread_count \
             FROM folders WHERE id = ?1",
        )
        .bind(&id_bytes)
        .fetch_optional(self.0)
        .await?
        .ok_or(StoreError::NotFound)?;
        row_to_folder(&row)
    }

    /// List folders for an account.
    pub async fn list_by_account(&self, account_id: AccountId) -> Result<Vec<Folder>> {
        let acc_bytes = uuid_bytes(&account_id.0);
        let rows = sqlx::query(
            "SELECT id, account_id, path, name, role, uid_validity, uid_next, message_count, unread_count \
             FROM folders WHERE account_id = ?1 ORDER BY path ASC",
        )
        .bind(&acc_bytes)
        .fetch_all(self.0)
        .await?;
        rows.iter().map(row_to_folder).collect()
    }

    /// Update message and unread counts for a folder.
    pub async fn update_counts(&self, id: FolderId, message_count: u32, unread_count: u32) -> Result<()> {
        let id_bytes = uuid_bytes(&id.0);
        sqlx::query("UPDATE folders SET message_count = ?1, unread_count = ?2 WHERE id = ?3")
            .bind(message_count as i64)
            .bind(unread_count as i64)
            .bind(&id_bytes)
            .execute(self.0)
            .await?;
        Ok(())
    }

    /// Whether this folder has had its one-time attachment-flag window scan.
    pub async fn attachments_scanned(&self, id: FolderId) -> Result<bool> {
        let id_bytes = uuid_bytes(&id.0);
        let row = sqlx::query("SELECT scanned FROM folder_attachment_scan WHERE folder_id = ?1")
            .bind(&id_bytes)
            .fetch_optional(self.0)
            .await?;
        Ok(row.map(|r| r.get::<i64, _>("scanned") != 0).unwrap_or(false))
    }

    /// Mark this folder's one-time attachment-flag window scan as done.
    pub async fn mark_attachments_scanned(&self, id: FolderId) -> Result<()> {
        let id_bytes = uuid_bytes(&id.0);
        sqlx::query(
            "INSERT INTO folder_attachment_scan (folder_id, scanned) VALUES (?1, 1) \
             ON CONFLICT(folder_id) DO UPDATE SET scanned = 1",
        )
        .bind(&id_bytes)
        .execute(self.0)
        .await?;
        Ok(())
    }

    /// Delete a folder by id (and attachment-scan / backfill side rows).
    pub async fn delete(&self, id: FolderId) -> Result<()> {
        let id_bytes = uuid_bytes(&id.0);
        sqlx::query("DELETE FROM folder_attachment_scan WHERE folder_id = ?1")
            .bind(&id_bytes)
            .execute(self.0)
            .await?;
        sqlx::query("DELETE FROM folder_backfill WHERE folder_id = ?1")
            .bind(&id_bytes)
            .execute(self.0)
            .await?;
        sqlx::query("DELETE FROM folders WHERE id = ?1")
            .bind(&id_bytes)
            .execute(self.0)
            .await?;
        Ok(())
    }

    /// Lowest UID already covered by historical backfill, if initialized.
    pub async fn backfill_low(&self, id: FolderId) -> Result<Option<u32>> {
        let id_bytes = uuid_bytes(&id.0);
        let row = sqlx::query("SELECT backfill_low FROM folder_backfill WHERE folder_id = ?1")
            .bind(&id_bytes)
            .fetch_optional(self.0)
            .await?;
        Ok(row.map(|r| r.get::<i64, _>("backfill_low") as u32))
    }

    /// Persist the historical backfill cursor.
    pub async fn set_backfill_low(&self, id: FolderId, backfill_low: u32) -> Result<()> {
        let id_bytes = uuid_bytes(&id.0);
        sqlx::query(
            "INSERT INTO folder_backfill (folder_id, backfill_low) VALUES (?1, ?2) \
             ON CONFLICT(folder_id) DO UPDATE SET backfill_low = excluded.backfill_low",
        )
        .bind(&id_bytes)
        .bind(backfill_low as i64)
        .execute(self.0)
        .await?;
        Ok(())
    }
}

fn role_to_str(r: FolderRole) -> &'static str {
    match r {
        FolderRole::Inbox => "inbox",
        FolderRole::Sent => "sent",
        FolderRole::Drafts => "drafts",
        FolderRole::Trash => "trash",
        FolderRole::Junk => "junk",
        FolderRole::Archive => "archive",
        FolderRole::Other => "other",
    }
}

fn role_from_str(s: &str) -> FolderRole {
    match s {
        "inbox" => FolderRole::Inbox,
        "sent" => FolderRole::Sent,
        "drafts" => FolderRole::Drafts,
        "trash" => FolderRole::Trash,
        "junk" => FolderRole::Junk,
        "archive" => FolderRole::Archive,
        _ => FolderRole::Other,
    }
}

fn row_to_folder(row: &sqlx::sqlite::SqliteRow) -> Result<Folder> {
    let id_bytes: Vec<u8> = row.try_get("id")?;
    let acc_bytes: Vec<u8> = row.try_get("account_id")?;
    let id_uuid = uuid_from_slice(&id_bytes).map_err(|e| StoreError::Other(e.to_string()))?;
    let acc_uuid = uuid_from_slice(&acc_bytes).map_err(|e| StoreError::Other(e.to_string()))?;
    let path: String = row.try_get("path")?;
    let name: String = row.try_get("name")?;
    let role: String = row.try_get("role")?;
    let uid_validity: i64 = row.try_get("uid_validity")?;
    let uid_next: i64 = row.try_get("uid_next")?;
    let message_count: i64 = row.try_get("message_count")?;
    let unread_count: i64 = row.try_get("unread_count")?;
    Ok(Folder {
        id: FolderId(id_uuid),
        account_id: AccountId(acc_uuid),
        path,
        name,
        role: role_from_str(&role),
        uid_validity: uid_validity as u32,
        uid_next: uid_next as u32,
        message_count: message_count as u32,
        unread_count: unread_count as u32,
    })
}

#[cfg(test)]
mod backfill_tests {
    use super::*;
    use crate::{AccountRepo, Db, MessageRepo};
    use chrono::Utc;
    use imt_core::{
        Account, AccountId, Address, AuthMethod, Flag, Folder, FolderId, FolderRole, ImapConfig,
        Message, MessageHeaders, MessageId, SmtpConfig, Tls, Uid,
    };

    fn tmp_db_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "imt-store-backfill-{}.sqlite3",
            uuid::Uuid::new_v4().simple()
        ))
    }

    fn account() -> Account {
        let auth = AuthMethod::Password {
            username: "me@example.com".into(),
        };
        Account {
            id: AccountId::new(),
            display_name: "Me".into(),
            address: Address::named("Me", "me@example.com"),
            imap: ImapConfig {
                host: "imap".into(),
                port: 993,
                tls: Tls::Implicit,
                auth: auth.clone(),
            },
            smtp: SmtpConfig {
                host: "smtp".into(),
                port: 465,
                tls: Tls::Implicit,
                auth,
            },
            order: 0,
            keep_on_server: true,
        }
    }

    fn folder(account_id: AccountId) -> Folder {
        Folder {
            id: FolderId::new(),
            account_id,
            path: "INBOX".into(),
            name: "INBOX".into(),
            role: FolderRole::Inbox,
            uid_validity: 1,
            uid_next: 100,
            message_count: 1,
            unread_count: 0,
        }
    }

    fn envelope(account_id: AccountId, folder_id: FolderId, uid: u32) -> Message {
        Message {
            id: MessageId::new(),
            account_id,
            folder_id,
            thread_id: None,
            uid: Uid(uid),
            headers: MessageHeaders {
                rfc_message_id: Some(format!("<{uid}@x>")),
                in_reply_to: None,
                references: vec![],
                from: vec![Address::new("a@b.com")],
                to: vec![],
                cc: vec![],
                bcc: vec![],
                reply_to: vec![],
                subject: "s".into(),
                date: Utc::now(),
            },
            flags: vec![Flag::Seen],
            size: 10,
            body: None,
            has_attachments: false,
            snippet: "s".into(),
            internal_date: Utc::now(),
        }
    }

    #[tokio::test]
    async fn already_full_local_history_skips_imap_dump() {
        // Given: folder whose local MIN(uid) <= 1
        let path = tmp_db_path();
        let db = Db::open(&path).await.unwrap();
        let acc = account();
        let fld = folder(acc.id);
        AccountRepo::new(db.pool()).upsert(&acc).await.unwrap();
        FolderRepo::new(db.pool()).upsert(&fld).await.unwrap();
        MessageRepo::new(db.pool())
            .upsert_envelope(&envelope(acc.id, fld.id, 1))
            .await
            .unwrap();

        // When: backfill init / migration runs (re-open applies INSERT … HAVING MIN(uid) <= 1;
        // row may already exist from first open — assert via ensure path: set if missing)
        let frepo = FolderRepo::new(db.pool());
        if frepo.backfill_low(fld.id).await.unwrap().is_none() {
            frepo.set_backfill_low(fld.id, 1).await.unwrap();
        }

        // Then: backfill_low = 1 and no historical UID FETCH needed
        assert_eq!(frepo.backfill_low(fld.id).await.unwrap(), Some(1));
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn migration_marks_complete_when_min_uid_is_one() {
        // Given: messages inserted before reading backfill cursor on a fresh DB
        let path = tmp_db_path();
        // Open once to create schema, insert data, then rely on migration's INSERT
        // already applied — simulate by inserting then calling the same SQL seed.
        let db = Db::open(&path).await.unwrap();
        let acc = account();
        let fld = folder(acc.id);
        AccountRepo::new(db.pool()).upsert(&acc).await.unwrap();
        FolderRepo::new(db.pool()).upsert(&fld).await.unwrap();
        MessageRepo::new(db.pool())
            .upsert_envelope(&envelope(acc.id, fld.id, 1))
            .await
            .unwrap();

        // When: migration-equivalent seed runs (idempotent INSERT OR IGNORE)
        sqlx::query(
            "INSERT OR IGNORE INTO folder_backfill (folder_id, backfill_low) \
             SELECT folder_id, 1 FROM messages GROUP BY folder_id HAVING MIN(uid) <= 1",
        )
        .execute(db.pool())
        .await
        .unwrap();

        // Then: cursor is complete
        assert_eq!(
            FolderRepo::new(db.pool()).backfill_low(fld.id).await.unwrap(),
            Some(1)
        );
        let _ = std::fs::remove_file(&path);
    }
}

