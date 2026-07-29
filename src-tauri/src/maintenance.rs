use sqlx::SqlitePool;
use std::time::Duration;
use tokio::sync::watch;

/// Interval between periodic WAL checkpoints.
pub const WAL_CHECKPOINT_INTERVAL: Duration = Duration::from_secs(300);

/// Runs a single WAL checkpoint in PASSIVE mode. PASSIVE checkpoints as many WAL
/// frames as it can without waiting on locks held by readers or writers, so it
/// never blocks the rest of the app — unlike RESTART/TRUNCATE, which block until
/// all readers finish.
pub async fn checkpoint_once(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("PRAGMA wal_checkpoint(PASSIVE)")
        .execute(pool)
        .await?;
    Ok(())
}

/// Periodically checkpoints the WAL until signaled to shut down via `shutdown_rx`.
pub async fn run_wal_checkpoint_loop(pool: SqlitePool, mut shutdown_rx: watch::Receiver<bool>) {
    let mut interval = tokio::time::interval(WAL_CHECKPOINT_INTERVAL);
    interval.tick().await; // skip immediate first tick
    loop {
        tokio::select! {
            _ = interval.tick() => {
                match checkpoint_once(&pool).await {
                    Ok(_) => tracing::debug!("WAL checkpoint complete"),
                    Err(e) => tracing::warn!("WAL checkpoint failed: {}", e),
                }
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    tracing::info!("WAL checkpoint task shutting down");
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_test_db;

    #[tokio::test]
    async fn checkpoint_once_succeeds() {
        let pool = open_test_db().await;
        assert!(checkpoint_once(&pool).await.is_ok());
    }

    #[tokio::test]
    async fn run_wal_checkpoint_loop_exits_on_shutdown_signal() {
        let pool = open_test_db().await;
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let handle = tokio::spawn(run_wal_checkpoint_loop(pool, shutdown_rx));
        shutdown_tx.send(true).expect("send shutdown signal");

        tokio::time::timeout(Duration::from_secs(2), handle)
            .await
            .expect("loop should exit promptly after shutdown signal")
            .expect("task should not panic");
    }
}
