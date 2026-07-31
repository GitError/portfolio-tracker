//! `build_portfolio_snapshot` lives in `portfolio-core` so it stays in sync
//! with the desktop app (`src-tauri`) — see #615. Tests for this function
//! live there too.
pub use portfolio_core::snapshot::build_portfolio_snapshot;
