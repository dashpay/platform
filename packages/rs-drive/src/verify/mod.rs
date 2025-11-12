#![allow(clippy::result_large_err)] // Errors intentionally carry rich context in verify paths
                                    // TODO: Revisit after shrinking top-level Error by boxing heavy variants
///DataContract verification methods on proofs
pub mod contract;
/// Document verification methods on proofs
pub mod document;
/// Identity verification methods on proofs
pub mod identity;
/// Single Document verification methods on proofs
pub mod single_document;

/// System components (Epoch info etc...) verification methods on proofs
pub mod system;

/// Group proof verification module
pub mod group;
/// Verifies that a state transition contents exist in the proof
pub mod state_transition;
/// Token proof verification module
pub mod tokens;
/// Voting proof verification module
pub mod voting;

/// Represents the root hash of the grovedb tree
pub type RootHash = [u8; 32];
