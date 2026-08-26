//! Verifies grovedb proofs produced by the having-range
//! (`GROUP BY … HAVING <aggregate> <op> <value> LIMIT n`) query surface.
//!
//! Mirrors the layering of [`super::document_ranked`]: a pure
//! grovedb-level verifier as a method on
//! [`DriveDocumentHavingQuery`](crate::query::DriveDocumentHavingQuery)
//! taking raw `proof: &[u8]` and returning `(RootHash, T)`. The
//! tenderdash signature composition that wraps this call lives in
//! `rs-drive-proof-verifier`.
//!
//! Only one verifier exists here, for the same reason as on the ranked
//! surface: every having-range request — any of the three axes, either
//! direction, any contiguous bound — resolves to one bounded-axis
//! `PathQuery` proved through grovedb's unified `prove_query`, differing
//! only in the axis, bounds, direction and limit the query carries.

/// Indexed-axis range proof verification — returns the groups the proof
/// commits to as falling inside the bound, in axis order.
pub mod verify_having_range_proof;
