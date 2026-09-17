use bincode::{Decode, Encode};
use std::fmt;

/// Version 0 of the persisted position of a paged eligibility walk.
#[derive(Debug, PartialEq, Eq, Clone, Default, Encode, Decode)]
pub struct ReadinessScanCursorV0 {
    /// The core height whose masternode list is the membership view of the walk.
    pub core_height: u32,
    /// The number of eligible evonodes in that view; a different count means a different view.
    pub hpmn_len: u32,
    /// The last report key examined; the next page starts after it.
    pub next_pro_tx_hash: Option<[u8; 32]>,
    /// Reports found eligible so far.
    pub eligible_so_far: u32,
    /// Reports pruned so far.
    pub pruned_so_far: u32,
    /// Reports examined so far.
    pub examined_so_far: u32,
}

impl fmt::Display for ReadinessScanCursorV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ReadinessScanCursorV0 {{ core_height: {}, hpmn_len: {}, next_pro_tx_hash: {:?}, eligible_so_far: {}, pruned_so_far: {}, examined_so_far: {} }}",
            self.core_height,
            self.hpmn_len,
            self.next_pro_tx_hash.map(hex::encode),
            self.eligible_so_far,
            self.pruned_so_far,
            self.examined_so_far
        )
    }
}
