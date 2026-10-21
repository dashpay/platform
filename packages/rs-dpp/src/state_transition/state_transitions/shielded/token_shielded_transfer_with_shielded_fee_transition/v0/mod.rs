mod state_transition_like;
mod state_transition_validation;
mod types;
pub(super) mod v0_methods;
mod version;

use crate::data_contract::TokenContractPosition;
use crate::fee::Credits;
use crate::prelude::Identifier;
use crate::shielded::SerializedAction;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize, PlatformSignable,
};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

/// Token shielded transfer paid from the credit pool, version 0.
///
/// A transfer inside a token's shielded pool whose fee is paid out of the credit shielded pool: an Orchard spend bundle in the token pool (value balance zero) and one in the credit pool (value balance the fee), both authorized by spend keys. No identity signs or is named anywhere.
///
/// The token bundle binds the token id and the transition's transparent fields into its
/// sighash; the fee bundle binds the token id and a digest of the token bundle's actions, so
/// neither bundle can be paired with another.
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
pub struct TokenShieldedTransferWithShieldedFeeTransitionV0 {
    /// The contract defining the token.
    pub data_contract_id: Identifier,
    /// The token's position in the contract.
    pub token_contract_position: TokenContractPosition,
    /// The token id, `calculate_token_id(data_contract_id, token_contract_position)`.
    pub token_id: Identifier,
    /// Orchard actions of the bundle in the token's shielded pool.
    pub token_actions: Vec<SerializedAction>,
    /// Sinsemilla root of the token pool's note commitment tree the token bundle was built against.
    pub token_anchor: [u8; 32],
    /// Halo 2 proof of the token bundle.
    pub token_proof: Vec<u8>,
    /// RedPallas binding signature of the token bundle.
    pub token_binding_signature: [u8; 64],
    /// Orchard actions of the spend bundle in the credit shielded pool that pays the fee.
    pub fee_actions: Vec<SerializedAction>,
    /// Sinsemilla root of the credit pool's note commitment tree the fee bundle was built against.
    pub fee_anchor: [u8; 32],
    /// Halo 2 proof of the fee bundle.
    pub fee_proof: Vec<u8>,
    /// RedPallas binding signature of the fee bundle.
    pub fee_binding_signature: [u8; 64],
    /// Credits leaving the credit pool: the fee bundle's value balance.
    pub credit_amount: Credits,
}
