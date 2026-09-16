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
