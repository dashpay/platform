pub(super) mod average_proof_helpers;
pub(super) mod count_proof_helpers;
/// `Fetch` impl for the average-side aggregate result. Returns
/// `(count, sum)`; client divides.
pub mod document_average;
pub mod document_count;
pub mod document_history_query;
pub mod document_query;
/// `Fetch` impl for the ranked (`HAVING … TOP(n)`) result — one entry
/// per returned group, in ranking order. Requires an index declaring
/// `rankedCountable` / `rankedSummable` / `rankedAverageable`
/// (protocol version 14+).
pub mod document_ranked_entries;
/// `Fetch` impl for the average-side per-entry result. Mirrors
/// `document_split_sums`.
pub mod document_split_averages;
pub mod document_split_counts;
/// `Fetch` impl for the sum-side per-entry result. Mirrors
/// `document_split_counts`.
pub mod document_split_sums;
/// `Fetch` impl for the sum-side aggregate result. Mirrors
/// `document_count`. Lights up alongside grovedb PR 670.
pub mod document_sum;
pub(super) mod ranked_proof_helpers;
pub(super) mod sum_proof_helpers;
pub mod transitions;
