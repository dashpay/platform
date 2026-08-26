//! Verifies grovedb proofs produced by the ranked
//! (`GROUP BY … ORDER BY <aggregate> LIMIT n [OFFSET m]`) query surface.
//!
//! Mirrors the layering of [`super::document_count`]: a pure
//! grovedb-level verifier as a method on
//! [`DriveDocumentRankedQuery`](crate::query::DriveDocumentRankedQuery)
//! taking raw `proof: &[u8]` and returning `(RootHash, T)`. The
//! tenderdash signature composition that wraps this call lives in
//! `rs-drive-proof-verifier`.
//!
//! Only one verifier exists here, and by design: the ranked surface has a
//! single proof shape. Where the count surface has five verifiers because
//! five different grovedb primitives can answer a count, every ranked
//! request — either direction, at any offset, on any of the three axes —
//! resolves to one top-k axis `PathQuery` proved through grovedb's
//! unified `prove_query`, differing only in the `(axis, k, descending,
//! offset)` tuple the query carries.

/// Indexed-axis top-k proof verification — returns the ranked groups the
/// proof commits to, in ranking order.
pub mod verify_ranked_top_k_proof;
