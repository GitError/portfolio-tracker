use sqlx::SqlitePool;
use tauri::Manager;

use crate::error::AppError;

use super::{BackupLockState, DbState};

/// SQLite magic bytes: first 16 bytes of a valid SQLite database file.
const SQLITE_MAGIC: &[u8] = b"SQLite format 3\0";

/// Error returned when backup/restore is invoked while another instance of
/// either is already running — both operate on the same live DB file.
const LOCK_CONFLICT_MESSAGE: &str = "Backup or restore already in progress";

#[tauri::command]
pub async fn backup_database(
    app: tauri::AppHandle,
    state: tauri::State<'_, DbState>,
    lock_state: tauri::State<'_, BackupLockState>,
    destination_path: String,
) -> Result<String, AppError> {
    let _guard = lock_state
        .0
        .try_lock()
        .map_err(|_| AppError::Conflict(LOCK_CONFLICT_MESSAGE.to_string()))?;

    // Flush WAL to ensure the file on disk is complete before we copy it.
    {
        let pool = &state.0;
        sqlx::query("PRAGMA wal_checkpoint(FULL)")
            .execute(pool)
            .await
            .map_err(|e| format!("WAL checkpoint failed: {e}"))?;
    }

    let source = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve app data dir: {e}"))?
        .join(crate::config::DB_FILE_NAME);

    if !source.exists() {
        return Err(AppError::Validation(
            "Database file does not exist".to_string(),
        ));
    }

    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve app data dir: {e}"))?;

    // Resolve the destination path. If only a filename is provided (no
    // directory component), save the backup to the app data directory.
    // Absolute paths are accepted only if they resolve (after canonicalization)
    // to a path inside the app data directory — this prevents symlink-based
    // path traversal and writing backup files to arbitrary locations.
    let requested = std::path::PathBuf::from(&destination_path);
    let dest: std::path::PathBuf = if requested.is_absolute() {
        requested
    } else {
        app_data_dir.join(&requested)
    };

    // Canonicalize the app data dir (must exist).
    let canonical_app_dir =
        std::fs::canonicalize(&app_data_dir).map_err(|e| format!("Cannot resolve app dir: {e}"))?;
    // Canonicalize dest — if the file doesn't exist yet, canonicalize its parent
    // directory to resolve any symlinks. If the parent cannot be canonicalized
    // we return an error rather than falling back to a potentially non-canonical path,
    // which would defeat the path-traversal check below.
    let canonical_dest = if dest.exists() {
        std::fs::canonicalize(&dest).map_err(|e| format!("Cannot resolve destination path: {e}"))?
    } else {
        let parent = dest
            .parent()
            .ok_or("Destination path has no parent directory")?;
        let canonical_parent = if parent.as_os_str().is_empty() {
            canonical_app_dir.clone()
        } else {
            std::fs::canonicalize(parent)
                .map_err(|e| format!("Cannot resolve destination directory: {e}"))?
        };
        canonical_parent.join(dest.file_name().ok_or("Destination path has no filename")?)
    };
    if !canonical_dest.starts_with(&canonical_app_dir) {
        return Err(AppError::Validation(format!(
            "Backup destination must be inside the app data directory ({})",
            app_data_dir.display()
        )));
    }

    if let Some(parent) = dest.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Could not create destination directory: {e}"))?;
        }
    }

    atomic_copy(&source, &dest)?;

    Ok(dest.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn restore_database(
    app: tauri::AppHandle,
    state: tauri::State<'_, DbState>,
    lock_state: tauri::State<'_, BackupLockState>,
    source_path: String,
) -> Result<String, AppError> {
    let _guard = lock_state
        .0
        .try_lock()
        .map_err(|_| AppError::Conflict(LOCK_CONFLICT_MESSAGE.to_string()))?;

    // Verify the source file is a valid SQLite database.
    let src = std::fs::canonicalize(&source_path)
        .map_err(|e| format!("Cannot resolve backup path: {e}"))?;
    if !src.is_file() {
        return Err(AppError::Validation(
            "Backup path must point to a regular file".to_string(),
        ));
    }

    // Check SQLite magic bytes.
    let mut header = [0u8; 16];
    {
        use std::io::Read;
        let mut f =
            std::fs::File::open(&src).map_err(|e| format!("Cannot open backup file: {e}"))?;
        f.read_exact(&mut header)
            .map_err(|_| "File is too small to be a valid SQLite database".to_string())?;
    }
    if header != SQLITE_MAGIC {
        return Err(AppError::Validation(
            "The selected file is not a valid SQLite database".to_string(),
        ));
    }

    // Open the source file with sqlx to verify it has a holdings table.
    {
        use sqlx::Row;
        let verify_url = format!("sqlite:{}?mode=ro", src.to_string_lossy());
        let verify_pool = sqlx::SqlitePool::connect(&verify_url)
            .await
            .map_err(|e| format!("Cannot open backup as SQLite: {e}"))?;

        let integrity_row = sqlx::query("PRAGMA integrity_check")
            .fetch_one(&verify_pool)
            .await
            .map_err(|e| format!("Integrity check failed on backup: {e}"))?;
        let integrity_result: String = integrity_row.get(0);
        if integrity_result != "ok" {
            verify_pool.close().await;
            return Err(AppError::Validation(format!(
                "Integrity check failed on backup: {}",
                integrity_result
            )));
        }

        let count_row = sqlx::query(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='holdings'",
        )
        .fetch_one(&verify_pool)
        .await
        .map_err(|e| format!("Could not verify holdings table: {e}"))?;
        let has_holdings: bool = count_row.get::<i64, _>(0) > 0;
        verify_pool.close().await;

        if !has_holdings {
            return Err(AppError::Validation(
                "Backup file does not appear to be a portfolio database (no holdings table)"
                    .to_string(),
            ));
        }
    }

    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve app data dir: {e}"))?;

    let dest = app_data_dir.join(crate::config::DB_FILE_NAME);
    let wal_path = app_data_dir.join(format!("{}-wal", crate::config::DB_FILE_NAME));
    let shm_path = app_data_dir.join(format!("{}-shm", crate::config::DB_FILE_NAME));
    let bak_path = app_data_dir.join(format!("{}.bak", crate::config::DB_FILE_NAME));
    let tmp_path = app_data_dir.join(format!("{}.restore_tmp", crate::config::DB_FILE_NAME));

    quiesce_and_restore_file(
        &state.0, &src, &dest, &wal_path, &shm_path, &bak_path, &tmp_path,
    )
    .await?;

    Ok("Database restored. Please restart the app to apply changes.".to_string())
}

/// Copies `source` to `dest` without ever exposing a partially-written file
/// at `dest`. Stages the copy into a temp file in the same directory as
/// `dest`, then atomically renames it into place — a same-directory rename
/// is atomic, so `dest` is always either its prior contents or the complete
/// new copy, never a half-written file from a failed or interrupted copy.
///
/// Fixes the check-then-write gap in #643: the old code checked whether
/// `dest` existed while resolving the path, then separately wrote to it with
/// `std::fs::copy`, leaving a window where a crash or error mid-copy could
/// leave `dest` truncated or corrupted.
fn atomic_copy(source: &std::path::Path, dest: &std::path::Path) -> Result<(), AppError> {
    let mut tmp_name = dest
        .file_name()
        .ok_or("Destination path has no filename")?
        .to_os_string();
    tmp_name.push(".tmp");
    let tmp_dest = dest.with_file_name(tmp_name);

    if let Err(e) = std::fs::copy(source, &tmp_dest) {
        let _ = std::fs::remove_file(&tmp_dest);
        return Err(AppError::Validation(format!(
            "Failed to copy database: {e}"
        )));
    }

    if let Err(e) = std::fs::rename(&tmp_dest, dest) {
        let _ = std::fs::remove_file(&tmp_dest);
        return Err(AppError::Validation(format!(
            "Failed to finalize backup: {e}"
        )));
    }

    Ok(())
}

/// Quiesces `pool`, atomically replaces `dest` with `backup_src` on disk, and
/// verifies the restored file with an integrity check.
///
/// Merely checkpointing the WAL before the copy is not enough: pooled SQLite
/// connections keep their own schema cache and hold open handles to `dest`
/// and its `-wal`/`-shm` companions, so overwriting the file on disk while a
/// connection is still live risks the old WAL being replayed over the
/// restored data. Closing the pool removes every such handle before the copy
/// runs. The pool must not be used again after this returns — the caller's
/// "restart the app to apply changes" response covers re-opening it.
async fn quiesce_and_restore_file(
    pool: &SqlitePool,
    backup_src: &std::path::Path,
    dest: &std::path::Path,
    wal_path: &std::path::Path,
    shm_path: &std::path::Path,
    bak_path: &std::path::Path,
    tmp_path: &std::path::Path,
) -> Result<(), AppError> {
    // Flush and truncate the WAL so the live DB file on disk is fully
    // self-contained before we quiesce the pool.
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(pool)
        .await
        .map_err(|e| format!("WAL checkpoint failed: {e}"))?;

    // Drain and close every pooled connection so nothing can read or write
    // `dest` while we overwrite it out from under them.
    pool.close().await;

    // Before overwriting the live database, create a safety backup. If the
    // copy fails we abort immediately so the live data is never touched.
    if dest.exists() {
        std::fs::copy(dest, bak_path)
            .map_err(|e| format!("Could not create safety backup before restore: {e}"))?;
    }

    // Stage the backup into a temp file in the same directory as `dest` and
    // verify its integrity BEFORE touching the live database. `dest` is only
    // ever replaced by an atomic rename once the staged file is known-good,
    // so a crash mid-copy or a backup that fails verification never leaves
    // `dest` in a corrupted or partially-written state.
    std::fs::copy(backup_src, tmp_path)
        .map_err(|e| format!("Failed to stage restore file: {e}"))?;

    if let Err(e) = verify_sqlite_integrity(tmp_path).await {
        let _ = std::fs::remove_file(tmp_path);
        return Err(e);
    }

    // Same-filesystem rename is atomic: `dest` is either the untouched
    // original or the fully-staged, already-verified new file — never a
    // partial write.
    std::fs::rename(tmp_path, dest).map_err(|e| format!("Failed to restore database: {e}"))?;

    // Remove stale WAL and SHM companion files so the restored DB starts
    // clean and SQLite does not attempt to replay the old journal.
    if wal_path.exists() {
        std::fs::remove_file(wal_path)
            .map_err(|e| format!("Could not remove WAL file after restore: {e}"))?;
    }
    if shm_path.exists() {
        std::fs::remove_file(shm_path)
            .map_err(|e| format!("Could not remove SHM file after restore: {e}"))?;
    }

    Ok(())
}

/// Opens `path` read-only via a dedicated connection (not the app pool) and
/// returns an error unless `PRAGMA integrity_check` reports "ok".
async fn verify_sqlite_integrity(path: &std::path::Path) -> Result<(), AppError> {
    use sqlx::Row;

    let url = format!("sqlite:{}?mode=ro", path.to_string_lossy());
    let pool = sqlx::SqlitePool::connect(&url)
        .await
        .map_err(|e| format!("Cannot open restored database for integrity check: {e}"))?;

    let result = sqlx::query("PRAGMA integrity_check")
        .fetch_one(&pool)
        .await
        .map_err(|e| format!("Integrity check failed on restored database: {e}"));

    let row = match result {
        Ok(row) => row,
        Err(e) => {
            pool.close().await;
            return Err(e.into());
        }
    };
    let integrity_result: String = row.get(0);
    pool.close().await;

    if integrity_result != "ok" {
        return Err(AppError::Validation(format!(
            "Integrity check failed on restored database: {}",
            integrity_result
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::Row;
    use std::path::PathBuf;

    /// Creates a unique scratch directory under the OS temp dir for a single test,
    /// so tests can run in parallel without clobbering each other's DB files.
    struct ScratchDir(PathBuf);

    impl ScratchDir {
        fn new(label: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "portfolio-tracker-restore-test-{}-{}-{}",
                label,
                std::process::id(),
                uuid::Uuid::new_v4()
            ));
            std::fs::create_dir_all(&dir).expect("create scratch dir");
            ScratchDir(dir)
        }

        fn path(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for ScratchDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Opens (creating if needed) a file-backed WAL-mode SQLite pool with
    /// migrations applied, mirroring how the real app opens its DB.
    async fn open_file_db(path: &std::path::Path) -> SqlitePool {
        let url = format!("sqlite:{}", path.to_string_lossy());
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);
        let pool = SqlitePool::connect_with(options)
            .await
            .unwrap_or_else(|e| panic!("open file db at {}: {e}", url));
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("run migrations");
        pool
    }

    async fn set_marker(pool: &SqlitePool, value: &str) {
        sqlx::query("INSERT OR REPLACE INTO app_config (key, value) VALUES ('marker', ?)")
            .bind(value)
            .execute(pool)
            .await
            .expect("insert marker");
    }

    async fn get_marker(path: &std::path::Path) -> Option<String> {
        let url = format!("sqlite:{}?mode=ro", path.to_string_lossy());
        let pool = SqlitePool::connect(&url).await.expect("open for read");
        let row = sqlx::query("SELECT value FROM app_config WHERE key = 'marker'")
            .fetch_optional(&pool)
            .await
            .expect("select marker");
        pool.close().await;
        row.map(|r| r.get::<String, _>(0))
    }

    #[test]
    fn atomic_copy_replaces_dest_contents() {
        let scratch = ScratchDir::new("atomic-copy-happy-path");
        let source_path = scratch.path("source.db");
        let dest_path = scratch.path("dest.db");

        std::fs::write(&source_path, b"new contents").expect("write source");
        std::fs::write(&dest_path, b"old contents").expect("write dest");

        atomic_copy(&source_path, &dest_path).expect("atomic_copy should succeed");

        let contents = std::fs::read(&dest_path).expect("read dest");
        assert_eq!(contents, b"new contents");
        assert!(
            !scratch.path("dest.db.tmp").exists(),
            "temp staging file should be cleaned up (consumed by rename) after success"
        );
    }

    #[test]
    fn atomic_copy_never_touches_existing_dest_when_source_is_missing() {
        // Regression guard for #643: if the copy step fails, `dest` must be
        // left byte-for-byte untouched rather than partially overwritten.
        let scratch = ScratchDir::new("atomic-copy-missing-source");
        let source_path = scratch.path("does-not-exist.db");
        let dest_path = scratch.path("dest.db");

        std::fs::write(&dest_path, b"original contents").expect("write dest");

        let result = atomic_copy(&source_path, &dest_path);

        assert!(result.is_err(), "copy from a missing source must fail");
        let contents = std::fs::read(&dest_path).expect("read dest");
        assert_eq!(
            contents, b"original contents",
            "dest must remain untouched when the copy step fails"
        );
        assert!(
            !scratch.path("dest.db.tmp").exists(),
            "failed temp staging file should be cleaned up rather than left behind"
        );
    }

    #[tokio::test]
    async fn backup_lock_state_rejects_concurrent_acquisition() {
        // Regression guard for #642: a second backup/restore attempt while one
        // is already running must fail fast instead of racing on the DB file.
        let lock_state = BackupLockState::new();
        let _first_guard = lock_state
            .0
            .try_lock()
            .expect("first lock acquisition should succeed");

        assert!(
            lock_state.0.try_lock().is_err(),
            "a second concurrent lock acquisition must fail while the first is held"
        );
    }

    #[tokio::test]
    async fn backup_lock_state_allows_acquisition_after_release() {
        let lock_state = BackupLockState::new();
        {
            let _guard = lock_state
                .0
                .try_lock()
                .expect("first lock acquisition should succeed");
        }

        assert!(
            lock_state.0.try_lock().is_ok(),
            "lock should be acquirable again once the prior guard is dropped"
        );
    }

    #[tokio::test]
    async fn quiesce_and_restore_file_replaces_dest_with_backup_contents() {
        let scratch = ScratchDir::new("happy-path");
        let backup_path = scratch.path("backup.db");
        let dest_path = scratch.path("live.db");
        let wal_path = scratch.path("live.db-wal");
        let shm_path = scratch.path("live.db-shm");
        let bak_path = scratch.path("live.db.bak");
        let tmp_path = scratch.path("live.db.restore_tmp");

        let backup_pool = open_file_db(&backup_path).await;
        set_marker(&backup_pool, "from-backup").await;
        backup_pool.close().await;

        let live_pool = open_file_db(&dest_path).await;
        set_marker(&live_pool, "from-live").await;

        quiesce_and_restore_file(
            &live_pool,
            &backup_path,
            &dest_path,
            &wal_path,
            &shm_path,
            &bak_path,
            &tmp_path,
        )
        .await
        .expect("restore should succeed");

        assert_eq!(
            get_marker(&dest_path).await.as_deref(),
            Some("from-backup"),
            "dest should now contain the backup's data"
        );
        assert!(
            !tmp_path.exists(),
            "temp staging file should be cleaned up (consumed by rename) after a successful restore"
        );
    }

    #[tokio::test]
    async fn quiesce_and_restore_file_closes_the_pool() {
        let scratch = ScratchDir::new("closes-pool");
        let backup_path = scratch.path("backup.db");
        let dest_path = scratch.path("live.db");
        let wal_path = scratch.path("live.db-wal");
        let shm_path = scratch.path("live.db-shm");
        let bak_path = scratch.path("live.db.bak");
        let tmp_path = scratch.path("live.db.restore_tmp");

        let backup_pool = open_file_db(&backup_path).await;
        backup_pool.close().await;

        let live_pool = open_file_db(&dest_path).await;

        quiesce_and_restore_file(
            &live_pool,
            &backup_path,
            &dest_path,
            &wal_path,
            &shm_path,
            &bak_path,
            &tmp_path,
        )
        .await
        .expect("restore should succeed");

        assert!(
            live_pool.is_closed(),
            "pool must be closed so no connection can write over the restored file before restart"
        );
    }

    #[tokio::test]
    async fn quiesce_and_restore_file_writes_safety_backup_of_prior_dest() {
        let scratch = ScratchDir::new("safety-backup");
        let backup_path = scratch.path("backup.db");
        let dest_path = scratch.path("live.db");
        let wal_path = scratch.path("live.db-wal");
        let shm_path = scratch.path("live.db-shm");
        let bak_path = scratch.path("live.db.bak");
        let tmp_path = scratch.path("live.db.restore_tmp");

        let backup_pool = open_file_db(&backup_path).await;
        set_marker(&backup_pool, "from-backup").await;
        backup_pool.close().await;

        let live_pool = open_file_db(&dest_path).await;
        set_marker(&live_pool, "original-live-data").await;

        quiesce_and_restore_file(
            &live_pool,
            &backup_path,
            &dest_path,
            &wal_path,
            &shm_path,
            &bak_path,
            &tmp_path,
        )
        .await
        .expect("restore should succeed");

        assert!(
            bak_path.exists(),
            "safety backup of the live DB should be written"
        );
        assert_eq!(
            get_marker(&bak_path).await.as_deref(),
            Some("original-live-data"),
            "safety backup should preserve the pre-restore live data"
        );
    }

    #[tokio::test]
    async fn quiesce_and_restore_file_leaves_dest_intact_when_staged_file_fails_integrity_check() {
        // Regression guard for #634: the restore must stage the backup into a
        // temp file and verify it BEFORE touching `dest`. A backup that fails
        // verification (corrupted, truncated, or not actually SQLite) must
        // never be copied over the live database.
        let scratch = ScratchDir::new("failed-integrity");
        let backup_path = scratch.path("backup.db");
        let dest_path = scratch.path("live.db");
        let wal_path = scratch.path("live.db-wal");
        let shm_path = scratch.path("live.db-shm");
        let bak_path = scratch.path("live.db.bak");
        let tmp_path = scratch.path("live.db.restore_tmp");

        // Not a valid SQLite database — verify_sqlite_integrity must reject it.
        std::fs::write(&backup_path, b"not a real sqlite database at all")
            .expect("write bogus backup file");

        let live_pool = open_file_db(&dest_path).await;
        set_marker(&live_pool, "original-live-data").await;

        let result = quiesce_and_restore_file(
            &live_pool,
            &backup_path,
            &dest_path,
            &wal_path,
            &shm_path,
            &bak_path,
            &tmp_path,
        )
        .await;

        assert!(
            result.is_err(),
            "restore must fail when the staged file does not pass integrity check"
        );
        assert_eq!(
            get_marker(&dest_path).await.as_deref(),
            Some("original-live-data"),
            "dest must remain byte-for-byte untouched when verification fails before the rename"
        );
        assert!(
            !tmp_path.exists(),
            "the failed temp staging file should be cleaned up rather than left behind"
        );
    }

    #[tokio::test]
    async fn verify_sqlite_integrity_rejects_non_sqlite_file() {
        let scratch = ScratchDir::new("integrity-check");
        let bogus_path = scratch.path("not-a-db.db");
        std::fs::write(&bogus_path, b"not a sqlite file at all").expect("write bogus file");

        let result = verify_sqlite_integrity(&bogus_path).await;

        assert!(
            result.is_err(),
            "a non-SQLite file must fail the integrity check"
        );
    }

    #[tokio::test]
    async fn verify_sqlite_integrity_accepts_a_healthy_database() {
        let scratch = ScratchDir::new("integrity-check-healthy");
        let db_path = scratch.path("healthy.db");
        let pool = open_file_db(&db_path).await;
        pool.close().await;

        let result = verify_sqlite_integrity(&db_path).await;

        assert!(
            result.is_ok(),
            "a freshly migrated database should pass integrity check"
        );
    }
}
