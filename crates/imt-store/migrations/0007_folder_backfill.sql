-- Backward history cursor per folder (separate from folders.uid_next forward tip).
-- backfill_low = lowest UID already fetched; <= 1 means history complete.
-- Absence of a row means "not initialized" (seed on next sync / ensure).
CREATE TABLE IF NOT EXISTS folder_backfill (
    folder_id    BLOB PRIMARY KEY,
    backfill_low INTEGER NOT NULL
);

-- Existing DBs that already hold UID 1 (full local history) are complete.
INSERT OR IGNORE INTO folder_backfill (folder_id, backfill_low)
SELECT folder_id, 1
  FROM messages
 GROUP BY folder_id
HAVING MIN(uid) <= 1;

-- Partial local history: continue downward from the lowest stored UID.
INSERT OR IGNORE INTO folder_backfill (folder_id, backfill_low)
SELECT folder_id, MIN(uid)
  FROM messages
 GROUP BY folder_id
HAVING MIN(uid) > 1;
