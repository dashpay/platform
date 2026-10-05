#![allow(clippy::result_large_err)] // Errors intentionally carry rich context in verify paths
                                    // TODO: Revisit after shrinking top-level Error by boxing heavy variants
/// Chained document query (provable semi-join) verification methods on
/// proofs — two grovedb proofs verified as one composed statement.
pub mod chained_document;
/// Composite document query (page plus derived sub-queries)
/// verification methods on proofs — one merged proof verified as one
/// composed statement.
pub mod composite_document;
///DataContract verification methods on proofs
pub mod contract;
/// Document verification methods on proofs
pub mod document;
/// Document-count verification methods on proofs (the
/// `GetDocumentsCount` endpoint's prove-path verifiers).
pub mod document_count;
/// Having-range verification methods on proofs (the
/// `GROUP BY … HAVING <aggregate> <op> <value> LIMIT n` surface's
/// prove-path verifier).
pub mod document_having;
/// Document-ranked verification methods on proofs (the
/// `GROUP BY … ORDER BY <aggregate> LIMIT n` surface's prove-path
/// verifier).
pub mod document_ranked;
/// Document-sum verification methods on proofs (the
/// `GetDocumentsSum` endpoint's prove-path verifiers).
pub mod document_sum;
/// Identity verification methods on proofs
pub mod identity;
/// Single Document verification methods on proofs
pub mod single_document;

/// System components (Epoch info etc...) verification methods on proofs
pub mod system;

/// Address funds proof verification module
pub mod address_funds;
/// Contract group proof verification
pub mod contract_groups;
/// Contract moderation proofs: one identity's status and pages of a contract's lists.
pub mod contract_moderation;
/// Group proof verification module
pub mod group;
/// Shielded pool proof verification module
pub mod shielded;
/// Verifies that a state transition contents exist in the proof
pub mod state_transition;
/// Token proof verification module
pub mod tokens;
/// Voting proof verification module
pub mod voting;

mod bounded_decode;

/// Represents the root hash of the grovedb tree
pub type RootHash = [u8; 32];

/// The root hash a range-total proof reconstructs when the tree it totals
/// (the last key of `path`, or a key above it) does not exist, or `None` when
/// the proof does not show that. grovedb proves such a path query by proving
/// the missing key absent and descending no further, which its aggregate
/// verifiers reject as a missing layer. The same proof verifies as a plain
/// query for the path's last key, and comes back empty only when a key on the
/// path is missing: a tree that exists comes with the lower layer the
/// aggregate read proved, which a plain key query refuses. So `Some` proves
/// the range holds nothing, and its total is zero.
///
/// The range-total verifiers fall back to it when grovedb refuses the proof.
/// They are selected by every protocol version with range counts and sums,
/// and the prover is unchanged: such a proof failed to verify before, and
/// verifies to a zero total now.
pub(crate) fn verify_absent_range_tree(
    proof: &[u8],
    path: &[Vec<u8>],
    platform_version: &dpp::version::PlatformVersion,
) -> Option<RootHash> {
    let (key, parent) = path.split_last()?;
    let absent = grovedb::PathQuery::new_single_key(parent.to_vec(), key.clone());
    match grovedb::GroveDb::verify_query(proof, &absent, &platform_version.drive.grove_version) {
        Ok((root_hash, elements)) if elements.is_empty() => Some(root_hash),
        _ => None,
    }
}
