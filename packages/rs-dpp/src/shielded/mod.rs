use bincode::{Decode, Encode};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

/// Parameters for a shielded pool, stored as an Item in the pool's subtree
#[derive(Debug, Clone, Encode, Decode, Default, PartialEq)]
pub struct ShieldedPoolParams {
    /// Counter for commitment tree checkpoint IDs (monotonically increasing)
    pub checkpoint_id_counter: u64,
}

/// A serialized Orchard action extracted from a bundle.
///
/// Each action represents one spend-and-output pair. The fields are raw bytes
/// suitable for serialization. Validation code reconstructs orchard types from
/// these bytes using grovedb-commitment-tree re-exports.
#[derive(Debug, Clone, Encode, Decode, PartialEq)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct SerializedAction {
    /// Nullifier of the spent note (Nullifier::to_bytes())
    pub nullifier: [u8; 32],
    /// Randomized spend validating key
    pub rk: [u8; 32],
    /// Extracted note commitment (ExtractedNoteCommitment::to_bytes())
    pub cmx: [u8; 32],
    /// Encrypted note ciphertext (epk + enc + out from TransmittedNoteCiphertext)
    pub encrypted_note: Vec<u8>,
    /// Value commitment (cv_net bytes)
    pub cv_net: [u8; 32],
    /// RedPallas spend authorization signature (64 bytes)
    pub spend_auth_sig: Vec<u8>,
}
