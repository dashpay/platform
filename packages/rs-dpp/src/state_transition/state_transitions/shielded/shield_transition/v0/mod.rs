mod state_transition_like;
mod state_transition_validation;
mod types;
#[cfg(feature = "state-transition-signing")]
pub(super) mod v0_methods;
mod version;

use std::collections::BTreeMap;

use crate::address_funds::{AddressFundsFeeStrategy, AddressWitness, PlatformAddress};
use crate::fee::Credits;
use crate::prelude::{AddressNonce, UserFeeIncrease};
use crate::shielded::SerializedAction;
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
pub struct ShieldTransitionV0 {
    /// Address inputs funding the shield (address -> nonce + max contribution).
    /// The total across all inputs must cover |value_balance| + fees.
    /// Excess credits remain in the source addresses.
    pub inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    /// Orchard actions (spend-output pairs)
    pub actions: Vec<SerializedAction>,
    /// Bundle flags (spends_enabled | outputs_enabled)
    pub flags: u8,
    /// Net value flowing into/out of the shielded pool
    pub value_balance: i64,
    /// Merkle root of the commitment tree at time of bundle creation
    pub anchor: [u8; 32],
    /// Halo2 proof bytes
    pub proof: Vec<u8>,
    /// RedPallas binding signature
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(with = "crate::shielded::serde_bytes_64")
    )]
    pub binding_signature: [u8; 64],
    /// Fee payment strategy
    pub fee_strategy: AddressFundsFeeStrategy,
    /// Fee multiplier
    pub user_fee_increase: UserFeeIncrease,
    /// Address witness signatures (excluded from sig hash)
    #[platform_signable(exclude_from_sig_hash)]
    pub input_witnesses: Vec<AddressWitness>,
}
