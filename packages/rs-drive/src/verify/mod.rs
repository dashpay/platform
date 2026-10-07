#![allow(clippy::result_large_err)] // Errors intentionally carry rich context in verify paths
                                    // TODO: Revisit after shrinking top-level Error by boxing heavy variants
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::{GroveDb, PathQuery};

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
/// The GroveDB proof envelope floor clients apply before a proof reaches Drive.
pub mod grovedb_proof_envelope;
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

/// A range total's verification from protocol version 14 on: `verified`, or,
/// when grovedb refused the proof, the empty total `empty` makes of the root
/// the proof reconstructs, if the proof shows the range holds nothing
/// ([`verify_empty_range_tree`] over the path of `range_path`'s query, the
/// query the prover proved); otherwise grovedb's refusal. The range-total
/// verifiers at version 1 wrap their version 0, which refuses such a proof as
/// released, in it.
pub(crate) fn or_empty_range_total<V>(
    verified: Result<V, Error>,
    proof: &[u8],
    range_path: impl FnOnce() -> Result<PathQuery, Error>,
    empty: impl FnOnce(RootHash) -> V,
    platform_version: &PlatformVersion,
) -> Result<V, Error> {
    match verified {
        Ok(verified) => Ok(verified),
        Err(error) => {
            let path_query = range_path()?;
            match verify_empty_range_tree(proof, &path_query.path, platform_version) {
                Some(root_hash) => Ok(empty(root_hash)),
                None => Err(error),
            }
        }
    }
}

/// The root hash a range-total proof reconstructs when the range it totals
/// holds nothing, or `None` when the proof does not show that: a key on
/// `path` is missing (an equality value no document holds), or the path's
/// last key holds an empty tree. grovedb proves a missing key absent and
/// descends no further, and proves an empty tree of a kind the read does not
/// aggregate (a provable sum tree under a count read, say) without a lower
/// layer; its aggregate verifiers reject both as a missing layer. The same
/// proof verifies as a plain query for the path's last key, which then holds
/// no element, or the empty tree itself, its hash bound to the empty tree's:
/// a tree holding entries comes with the lower layer the aggregate read
/// proved, which a plain key query refuses, or is not empty. So `Some` proves
/// the range holds nothing, and its total is zero.
fn verify_empty_range_tree(
    proof: &[u8],
    path: &[Vec<u8>],
    platform_version: &PlatformVersion,
) -> Option<RootHash> {
    let (key, parent) = path.split_last()?;
    let query = PathQuery::new_single_key(parent.to_vec(), key.clone());
    let (root_hash, elements) =
        GroveDb::verify_query(proof, &query, &platform_version.drive.grove_version).ok()?;
    match elements.as_slice() {
        [] => Some(root_hash),
        [(_, _, Some(element))] if element.is_any_tree() && !element.is_non_empty_tree() => {
            Some(root_hash)
        }
        _ => None,
    }
}
