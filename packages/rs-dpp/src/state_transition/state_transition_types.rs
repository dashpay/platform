use bincode::{Decode, Encode};
use num_enum::{IntoPrimitive, TryFromPrimitive};

use serde_repr::{Deserialize_repr, Serialize_repr};

#[repr(u8)]
#[derive(
    Serialize_repr,
    Deserialize_repr,
    PartialEq,
    Eq,
    Clone,
    Copy,
    Debug,
    TryFromPrimitive,
    IntoPrimitive,
    Encode,
    Decode,
    Default,
)]
pub enum StateTransitionType {
    #[default]
    DataContractCreate = 0,
    DocumentsBatch = 1,
    IdentityCreate = 2,
    IdentityTopUp = 3,
    DataContractUpdate = 4,
    IdentityUpdate = 5,
    IdentityCreditWithdrawal = 6,
    IdentityCreditTransfer = 7,
    MasternodeVote = 8,
}

impl std::fmt::Display for StateTransitionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
