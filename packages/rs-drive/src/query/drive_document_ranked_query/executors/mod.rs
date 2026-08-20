//! Per-mode ranked executors on `impl Drive`. One file per response
//! shape — the dispatcher ([`super::drive_dispatcher`]) picks between
//! them on the request's `prove` flag.
//!
//! Each executor resolves the covering index, builds the
//! [`DriveDocumentRankedQuery`], and runs the matching method on it. The
//! resolution step is shared ([`ranked_query_for_mode`]) rather than
//! duplicated per file: the no-proof and prove paths must agree on the
//! index — and therefore the grove path — or a client would verify a
//! proof about a different subtree than the one an unproven read
//! returned.
//!
//! No re-exports needed: each file adds methods directly to `impl Drive`.

pub mod top_k_no_proof;
pub mod top_k_proof;

// Resolution — covering-index pick + equality-pin encoding — is shared
// with the SDK's proof helpers through
// [`super::index_picker::resolve_ranked_query_for_mode`]: both sides
// must land on the same index and the same prefix segments, or a client
// would verify a proof about a different subtree than the one an
// unproven read returned.
pub(super) use super::index_picker::resolve_ranked_query_for_mode as ranked_query_for_mode;
