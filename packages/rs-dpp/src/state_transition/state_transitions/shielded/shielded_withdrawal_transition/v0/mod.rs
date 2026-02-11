mod state_transition_like;
mod state_transition_validation;
mod types;
mod v0_methods;
mod version;

use crate::identity::core_script::CoreScript;
use crate::prelude::UserFeeIncrease;
use crate::shielded::SerializedAction;
use crate::withdrawal::Pooling;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserialize,
    PlatformSignable,
    PartialEq,
)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[platform_serialize(unversioned)]
pub struct ShieldedWithdrawalTransitionV0 {
    /// Withdrawal amount in credits
    pub amount: u64,
    /// Orchard actions (spends + change outputs)
    pub actions: Vec<SerializedAction>,
    /// Bundle flags (spends_enabled | outputs_enabled)
    pub flags: u8,
    /// Net value balance (amount + fee flowing out of shielded pool)
    pub value_balance: i64,
    /// Merkle root of the commitment tree used for spends
    pub anchor: [u8; 32],
    /// Halo2 proof bytes
    pub proof: Vec<u8>,
    /// RedPallas binding signature
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(with = "crate::shielded::serde_bytes_64")
    )]
    pub binding_signature: [u8; 64],
    /// Core transaction fee rate
    pub core_fee_per_byte: u32,
    /// Withdrawal pooling strategy
    pub pooling: Pooling,
    /// Core address receiving withdrawn funds
    pub output_script: CoreScript,
    /// Fee multiplier
    pub user_fee_increase: UserFeeIncrease,
}
