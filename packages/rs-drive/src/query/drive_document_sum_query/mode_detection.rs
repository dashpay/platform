//! Sum-query mode detection — pure functions that classify the
//! request shape independently of dispatch / I/O. Parallels count's
//! `mode_detection.rs`.
//!
//! The dispatcher's `detect_sum_mode` (in `drive_dispatcher.rs`) calls
//! into these helpers. They're factored out here so unit tests can
//! drive them directly without spinning up a Drive instance.

// TODO(sum-feature): port count's `mode_detection.rs`. The functions
// to mirror are:
//   - `detect_mode(...)` — full mode resolution
//   - `is_range_operator(WhereOperator) -> bool` — already lifted to
//     `mod.rs` here so the verifier surface can reach it without
//     pulling in the dispatcher
//   - `is_indexable_for_count(...)` — already lifted as
//     `is_indexable_for_sum(...)` in `mod.rs`
//   - `merge_same_field_range_pairs(...)` — between* canonicalization;
//     identical contract on the sum side
//
// Keeping this file as a placeholder for the canonicalization helpers
// the dispatcher will pull in once executors land.
