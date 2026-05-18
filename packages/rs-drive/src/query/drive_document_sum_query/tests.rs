//! Unit tests for the sum-query surface. Stub for now — populated as
//! each executor lands.
//!
//! Test plan (parallels count's `drive_document_count_query/tests.rs`):
//!
//! - Total fast path: contract with `documents_summable: "amount"`,
//!   insert N documents, assert `Drive::execute_document_sum_request`
//!   returns `Aggregate(sum)` where sum equals the expected total
//!   given the bench's deterministic schedule.
//! - Per-recipient point lookup: `WHERE recipient == X` on the
//!   `byRecipient` index → `Aggregate(per_recipient_sum)`.
//! - Range: `WHERE sentAt > T` on the `bySentAt` index →
//!   `Aggregate(sum_in_range)`.
//! - Reject contract: missing `summable` index for the query's
//!   `sum_property` → `WhereClauseOnNonIndexedProperty`.
//! - Doctype mismatch: request's `sum_property` ≠ the doctype's
//!   `documents_summable` → typed rejection at parse time.
//! - All five tests get a prove/verify variant once
//!   `verify_aggregate_sum_query` lands in grovedb (PR 670).
