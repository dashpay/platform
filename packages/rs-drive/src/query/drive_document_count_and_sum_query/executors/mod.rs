//! Per-`DocumentSumMode` joint count-and-sum no-prove executors.
//!
//! Mirrors [`crate::query::drive_document_sum_query::executors`]
//! one-to-one for the no-prove subset of modes — the prove modes
//! (`RangeProof`, `RangeDistinctProof`, `PointLookupProof`,
//! `RangeAggregateCarrierProof`) are unreachable here because the AVG
//! dispatcher routes prove-true requests through
//! [`Drive::execute_document_average_prove`] before this module is
//! consulted. The three no-prove modes are:
//!
//! - [`total`] — `DocumentSumMode::Total` (empty-where +
//!   `documents_summable + documents_countable` fast path AND
//!   Equal/In-fully-covered point lookup).
//! - [`per_in_value`] — `DocumentSumMode::PerInValue` (Equal/In on a
//!   non-range covered index, emitting one entry per In branch).
//! - [`range_no_proof`] — `DocumentSumMode::RangeNoProof` (range and
//!   distinct shapes over a PCPS-eligible index).

pub mod per_in_value;
pub mod range_no_proof;
pub mod total;
