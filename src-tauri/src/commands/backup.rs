use tauri::Manager;

use crate::error::AppError;

use super::DbState;

/// SQLite magic bytes: first 16 bytes of a valid SQLite database file.
const SQLITE_MAGIC: &[u8] = b"SQLite format 3\0";

#[tauri::command]
pub async fn backup_database(
    app: tauri::AppHandle,
    state: tauri::State<'_, DbState>,
    destination_path: String,
) -> Result<String, AppError> {
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

    std::fs::copy(&source, &dest).map_err(|e| format!("Failed to copy database: {e}"))?;

    Ok(dest.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn restore_database(
    app: tauri::AppHandle,
    state: tauri::State<'_, DbState>,
    source_path: String,
) -> Result<String, AppError> {
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

    // Flush and truncate the WAL so the live DB file on disk is fully
    // self-contained before we overwrite it.  This prevents the old WAL from
    // being replayed over the newly restored data when the pool reconnects.
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(&state.0)
        .await
        .map_err(|e| format!("WAL checkpoint failed: {e}"))?;

    // Before overwriting the live database, create a safety backup.  If the
    // copy fails we abort immediately so the live data is never touched.
    if dest.exists() {
        let bak = app_data_dir.join(format!("{}.bak", crate::config::DB_FILE_NAME));
        std::fs::copy(&dest, &bak)
            .map_err(|e| format!("Could not create safety backup before restore: {e}"))?;
    }

    std::fs::copy(&src, &dest).map_err(|e| format!("Failed to restore database: {e}"))?;

    // Remove stale WAL and SHM companion files so the restored DB starts
    // clean and SQLite does not attempt to replay the old journal.
    let wal_path = app_data_dir.join(format!("{}-wal", crate::config::DB_FILE_NAME));
    let shm_path = app_data_dir.join(format!("{}-shm", crate::config::DB_FILE_NAME));
    if wal_path.exists() {
        std::fs::remove_file(&wal_path)
            .map_err(|e| format!("Could not remove WAL file after restore: {e}"))?;
    }
    if shm_path.exists() {
        std::fs::remove_file(&shm_path)
            .map_err(|e| format!("Could not remove SHM file after restore: {e}"))?;
    }

    Ok("Database restored. Please restart the app to apply changes.".to_string())
}
