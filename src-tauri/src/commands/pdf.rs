use std::path::{Path, PathBuf};

use chrono::Utc;
use tauri::State;

use crate::error::AppError;
use crate::pdf::build_portfolio_pdf;

use super::portfolio::get_portfolio_impl;
use super::{DbState, RealizedGainsCacheState};

/// Builds a PDF snapshot of the current portfolio and saves it to
/// `~/Downloads/portfolio-YYYY-MM-DD.pdf`, overwriting an existing file from the
/// same day (consistent with `backup_database`'s same-day behavior). No
/// destination path is accepted from the frontend — there is nothing here for
/// `tauri-plugin-dialog` to add and no user-supplied path to validate.
#[tauri::command]
pub async fn export_portfolio_pdf(
    db: State<'_, DbState>,
    gains_cache: State<'_, RealizedGainsCacheState>,
) -> Result<String, AppError> {
    let snapshot = get_portfolio_impl(&db.0, &gains_cache).await?;
    let pdf_bytes = build_portfolio_pdf(&snapshot).map_err(AppError::from)?;

    let home = dirs::home_dir()
        .ok_or_else(|| AppError::Validation("Could not resolve home directory".to_string()))?;
    let (dest, display_path) = resolve_pdf_destination(&home);
    write_pdf_file(&dest, &pdf_bytes).map_err(AppError::from)?;

    Ok(display_path)
}

/// Computes the save path and its display form from a home directory. Takes
/// `home` as a parameter (rather than calling `dirs::home_dir()` itself) so it
/// can be tested without touching the real filesystem or process environment.
fn resolve_pdf_destination(home: &Path) -> (PathBuf, String) {
    let filename = format!("portfolio-{}.pdf", Utc::now().format("%Y-%m-%d"));
    let dest = home.join("Downloads").join(&filename);
    // Displayed directly in the frontend's success toast — showing `~/Downloads/...`
    // instead of the resolved absolute path avoids leaking the full home directory.
    let display_path = format!("~/Downloads/{filename}");
    (dest, display_path)
}

fn write_pdf_file(dest: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create Downloads directory: {e}"))?;
    }
    std::fs::write(dest, bytes).map_err(|e| format!("Failed to write PDF: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;
    use std::path::PathBuf;

    struct ScratchDir(PathBuf);

    impl ScratchDir {
        fn new(label: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "portfolio-tracker-pdf-test-{}-{}-{}",
                label,
                std::process::id(),
                uuid::Uuid::new_v4()
            ));
            std::fs::create_dir_all(&dir).expect("create scratch dir");
            ScratchDir(dir)
        }
    }

    impl Drop for ScratchDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn resolve_pdf_destination_uses_todays_date_and_a_tilde_display_path() {
        let home = Path::new("/home/example-user");

        let (dest, display_path) = resolve_pdf_destination(home);

        let today = Utc::now().format("%Y-%m-%d");
        assert_eq!(
            dest,
            PathBuf::from(format!(
                "/home/example-user/Downloads/portfolio-{today}.pdf"
            ))
        );
        assert_eq!(display_path, format!("~/Downloads/portfolio-{today}.pdf"));
    }

    #[test]
    fn write_pdf_file_creates_the_downloads_directory_and_writes_the_file() {
        let scratch = ScratchDir::new("write-pdf-file");
        let dest = scratch.0.join("Downloads").join("portfolio-test.pdf");

        write_pdf_file(&dest, b"%PDF-1.7 fake pdf content").expect("write should succeed");

        assert!(dest.exists(), "file should exist on disk after the call");
        let contents = std::fs::read(&dest).expect("read written file");
        assert_eq!(contents, b"%PDF-1.7 fake pdf content");
    }

    #[test]
    fn write_pdf_file_overwrites_an_existing_file_at_the_same_path() {
        let scratch = ScratchDir::new("write-pdf-file-overwrite");
        let dest = scratch.0.join("Downloads").join("portfolio-test.pdf");
        write_pdf_file(&dest, b"old content").expect("first write should succeed");

        write_pdf_file(&dest, b"new content").expect("second write should succeed");

        let contents = std::fs::read(&dest).expect("read written file");
        assert_eq!(contents, b"new content");
    }

    async fn open_test_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("open in-memory db");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("run migrations");
        pool
    }

    #[tokio::test]
    async fn build_portfolio_pdf_succeeds_for_a_freshly_migrated_empty_portfolio() {
        let pool = open_test_db().await;
        let gains_cache = RealizedGainsCacheState::new();

        let snapshot = get_portfolio_impl(&pool, &gains_cache)
            .await
            .expect("empty portfolio snapshot should build");
        let pdf_bytes =
            build_portfolio_pdf(&snapshot).expect("PDF should build for an empty portfolio");

        assert!(pdf_bytes.starts_with(b"%PDF-"));
    }
}
