//! Compilation readiness storage: the rounds, reports, cursors and deadlines kept under
//! `[Votes] / r` and the readiness funds kept under `[PreFundedSpecializedBalances] / 129`.
//!
//! A round is stored under its own key beneath its contract so that replacing it is a pointer
//! swap plus fixed-size inserts and deletes, never a walk of its reports; a retired round is
//! unlinked from the pointer in one batch and drained later by the bounded cleanup step. See
//! the book chapter on compilation readiness for the layout and the reasons.

#[cfg(feature = "server")]
mod activate_readiness_round_operations;
#[cfg(feature = "server")]
mod cancel_readiness_round_operations;
#[cfg(feature = "server")]
mod cleanup_retired_readiness_round_operations;
#[cfg(feature = "server")]
mod clear_readiness_scan_cursor_operations;
#[cfg(feature = "server")]
mod estimation_costs;
#[cfg(feature = "server")]
mod fetch_readiness_evaluation_cursor;
#[cfg(feature = "server")]
mod fetch_readiness_reports_page;
#[cfg(feature = "server")]
mod fetch_readiness_round;
#[cfg(feature = "server")]
mod fetch_readiness_round_raw_count;
#[cfg(feature = "server")]
mod fetch_readiness_rounds_due;
#[cfg(feature = "server")]
mod fetch_readiness_rounds_page;
#[cfg(feature = "server")]
mod fetch_readiness_scan_cursor;
#[cfg(feature = "server")]
mod fetch_retired_readiness_round;
/// Readiness funds under `[PreFundedSpecializedBalances] / 129`
#[cfg(feature = "server")]
pub mod fund;
#[cfg(feature = "server")]
mod insert_readiness_report_operations;
#[cfg(feature = "server")]
mod open_readiness_round_operations;
#[cfg(feature = "server")]
mod prove_readiness_round;
#[cfg(feature = "server")]
mod prune_readiness_reports_operations;
/// Path queries shared by the prover and the verifier
#[cfg(any(feature = "server", feature = "verify"))]
pub mod queries;
#[cfg(feature = "server")]
mod record_readiness_crossing_operations;
#[cfg(feature = "server")]
mod retire_readiness_round_operations;
#[cfg(feature = "server")]
mod store_readiness_evaluation_cursor_operations;
#[cfg(feature = "server")]
mod store_readiness_scan_cursor_operations;
#[cfg(feature = "server")]
mod update_readiness_round_evaluation_operations;

#[cfg(feature = "server")]
pub use cleanup_retired_readiness_round_operations::ReadinessCleanupOutcome;
#[cfg(feature = "server")]
pub use fetch_readiness_rounds_due::ReadinessRoundDue;
#[cfg(feature = "server")]
pub use fetch_retired_readiness_round::RetiredReadinessRound;
#[cfg(feature = "server")]
pub use open_readiness_round_operations::ReadinessRoundFunding;
#[cfg(feature = "server")]
pub use retire_readiness_round_operations::ReadinessRetirement;

#[cfg(all(feature = "server", test))]
mod tests;
