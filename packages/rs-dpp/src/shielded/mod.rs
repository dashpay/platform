use bincode::{Decode, Encode};

/// Parameters for a shielded pool, stored as an Item in the pool's subtree
#[derive(Debug, Clone, Encode, Decode, Default, PartialEq)]
pub struct ShieldedPoolParams {
    /// Counter for commitment tree checkpoint IDs (monotonically increasing)
    pub checkpoint_id_counter: u64,
}
