//! End-to-end test modules for `insert_contract_operations_v0`.
//!
//! Extracted from `v0/mod.rs` (which kept ~4 000 lines of tests
//! inline alongside ~450 lines of impl) so the impl side stays
//! readable. Three submodules cover the three feature surfaces
//! the v0 contract apply path supports today:
//!
//! - [`countable_e2e_tests`] — document-type-level
//!   `documentsCountable` / `rangeCountable` (primary-key tree
//!   variant).
//! - [`range_countable_index_e2e_tests`] — per-index
//!   `rangeCountable: true` (property-name tree variant +
//!   `NonCounted`-wrapped continuations).
//! - [`range_summable_index_e2e_tests`] — per-index
//!   `rangeSummable` / `rangeCountable` 4-way dispatcher
//!   (`(false, false) | (true, false) | (false, true) | (true,
//!   true)` corners of the matrix).
//! - [`shared_prefix_aggregation_e2e_tests`] — cross-index shared
//!   prefix layouts where a shorter aggregate index and a deeper
//!   range index must compose safely during document insertion.
//! - [`ranked_index_e2e_tests`] — per-index `rankedCountable` /
//!   `rankedSummable` / `rankedAverageable` (meta schema v3 / PV14):
//!   the indexed-tree variants and their ordered secondaries, end to
//!   end through insert / update / delete. Its child module
//!   `batched_group_drain` lives in `batched_group_drain.rs` beside
//!   this file and is declared with `#[path]` so it can reuse that
//!   suite's fixture and assertion helpers.

mod countable_e2e_tests;
mod index_only_e2e_tests;
mod preallocated_index_e2e_tests;
mod range_countable_index_e2e_tests;
mod range_summable_index_e2e_tests;
mod ranked_index_e2e_tests;
mod shared_prefix_aggregation_e2e_tests;
