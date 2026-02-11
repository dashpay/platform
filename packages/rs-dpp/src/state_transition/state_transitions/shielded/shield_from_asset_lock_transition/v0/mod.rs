mod proved;
mod state_transition_like;
mod state_transition_validation;
mod types;
mod version;

use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
use crate::prelude::UserFeeIncrease;
use crate::shielded::SerializedAction;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_value::BinaryData;
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
pub struct ShieldFromAssetLockTransitionV0 {
    /// Asset lock proof from L1
    pub asset_lock_proof: AssetLockProof,
    /// Orchard actions (spend-output pairs)
    pub actions: Vec<SerializedAction>,
    /// Bundle flags (spends_enabled | outputs_enabled)
    pub flags: u8,
    /// Net value flowing into the shielded pool (must be negative for shielding)
    pub value_balance: i64,
    /// Merkle root of the commitment tree at time of bundle creation
    pub anchor: [u8; 32],
    /// Halo2 proof bytes
    pub proof: Vec<u8>,
    /// RedPallas binding signature
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(with = "crate::shielded::serde_bytes_64"))]
    pub binding_signature: [u8; 64],
    /// Fee multiplier
    pub user_fee_increase: UserFeeIncrease,
    /// ECDSA signature over the signable bytes (excluded from sig hash)
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}
