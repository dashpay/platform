mod state_transition_like;
mod state_transition_validation;
mod types;
#[cfg(feature = "state-transition-signing")]
pub(super) mod v1_methods;
mod version;

use std::collections::BTreeMap;

use crate::address_funds::{AddressFundsFeeStrategy, AddressWitness, PlatformAddress};
use crate::fee::Credits;
use crate::prelude::{AddressNonce, UserFeeIncrease};
use crate::shielded::SerializedAction;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize, PlatformSignable,
};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

/// The fields of version 0, with a signed format discriminant separating the
/// funding-bound bundle domain from the unbound bundles of protocol versions 12 and 13.
/// Version 0 is refused without charging once protocol version 14 activates.
#[cfg_attr(feature = "json-conversion", crate::serialization::json_safe_fields)]
#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    PlatformSignable,
    PartialEq,
    DecodeUntrusted,
)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[platform_serialize(unversioned)]
pub struct ShieldTransitionV1 {
    /// Address inputs funding the shield (address -> nonce + max contribution).
    /// The total across all inputs must cover |value_balance| + fees.
    /// Excess credits remain in the source addresses.
    #[cfg_attr(
        feature = "json-conversion",
        serde(with = "crate::address_funds::serde_helpers::address_input_map")
    )]
    pub inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    /// Orchard actions (spend-output pairs)
    pub actions: Vec<SerializedAction>,
    /// Amount of credits being shielded (entering the shielded pool).
    pub amount: u64,
    /// Sinsemilla root of the note commitment tree (Orchard Anchor)
    pub anchor: [u8; 32],
    /// Halo2 proof bytes
    pub proof: Vec<u8>,
    /// RedPallas binding signature
    pub binding_signature: [u8; 64],
    /// Fee payment strategy
    pub fee_strategy: AddressFundsFeeStrategy,
    /// Fee multiplier
    pub user_fee_increase: UserFeeIncrease,
    /// Address witness signatures (excluded from sig hash)
    #[platform_signable(exclude_from_sig_hash)]
    pub input_witnesses: Vec<AddressWitness>,
}
