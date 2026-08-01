//! `run_stress_test` lives in `portfolio-core` so it stays in sync with
//! `src-tauri` — see #645. Tests for this function (including the
//! cross-language parity guard for #646) live there too.
//!
//! A hand-maintained TypeScript port lives in `frontend/lib/scenarioMath.ts`
//! for use when no Tauri backend is available — keep it in sync with
//! `portfolio-core/src/stress.rs` if the shock model changes.
pub use portfolio_core::stress::{run_stress_test, validate_shocks};
