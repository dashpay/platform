pub(crate) mod average_proof_helpers;
pub(crate) mod count_proof_helpers;
pub mod document_average;
pub mod document_count;
pub mod document_having_entries;
pub mod document_history_query;
pub mod document_query;
pub mod document_ranked_entries;
pub mod document_split_averages;
pub mod document_split_counts;
pub mod document_split_sums;
pub mod document_sum;
pub(crate) mod having_proof_helpers;
/// Client-side wire-proto → drive-type decoders for `getDocuments`,
/// consumed by
/// [`document_query::DocumentQuery::try_from_request`]. They mirror
/// rs-drive-abci's server request decode and must be kept in
/// lockstep with it (see the module docs).
pub(crate) mod proto_conversions;
pub(crate) mod ranked_proof_helpers;
pub(crate) mod sum_proof_helpers;
