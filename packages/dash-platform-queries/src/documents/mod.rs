pub(crate) mod average_proof_helpers;
pub(crate) mod count_proof_helpers;
/// `FromProof` impl for the average-side aggregate result. Returns
/// `(count, sum)`; client divides.
pub mod document_average;
pub mod document_count;
pub mod document_history_query;
pub mod document_query;
/// `FromProof` impl for the ranked (`GROUP BY … ORDER BY <aggregate> LIMIT n
/// [OFFSET m]`) result — one entry per returned group, in ranking order,
/// plus the rank the page starts at. Requires an index declaring
/// `rankedCountable` / `rankedSummable` / `rankedAverageable`
/// (protocol version 14+).
pub mod document_ranked_entries;
/// `FromProof` impl for the average-side per-entry result. Mirrors
/// `document_split_sums`.
pub mod document_split_averages;
pub mod document_split_counts;
/// `FromProof` impl for the sum-side per-entry result. Mirrors
/// `document_split_counts`.
pub mod document_split_sums;
/// `FromProof` impl for the sum-side aggregate result. Mirrors
/// `document_count`. Lights up alongside grovedb PR 670.
pub mod document_sum;
pub(crate) mod ranked_proof_helpers;
pub(crate) mod sum_proof_helpers;
