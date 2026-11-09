mod state_transition_like;
mod state_transition_validation;
pub use state_transition_validation::validate_private_document_create_entries;
mod types;
pub(super) mod v0_methods;
mod version;

use crate::prelude::Identifier;
use crate::shielded::SerializedAction;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize, PlatformSignable,
};
use platform_value::BinaryData;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

/// Entries appended to a private document type's store, paid from the credit shielded pool,
/// version 0.
///
/// No platform signature: the Orchard proof and spend-auth signatures authorize the payment,
/// and the contract, the type name and every entry are committed into the Orchard binding
/// sighash (`private_document_create_from_shielded_pool_extra_sighash_data`), so the payment
/// can not be moved to other entries.
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
pub struct PrivateDocumentCreateFromShieldedPoolTransitionV0 {
    /// The contract that defines the private document type.
    pub data_contract_id: Identifier,
    /// The private document type the entries are appended to.
    pub document_type_name: String,
    /// The entries, appended in this order. Each takes exactly the type's `entrySize`.
    pub entries: Vec<BinaryData>,
    /// Orchard actions (spend-output pairs) of the credit pool bundle that pays the fee.
    pub actions: Vec<SerializedAction>,
    /// Credits leaving the credit pool, the bundle's value balance: exactly the fee
    /// (`compute_private_document_create_from_shielded_pool_fee`).
    pub value_balance: u64,
    /// Sinsemilla root of the note commitment tree the bundle was built against.
    pub anchor: [u8; 32],
    /// Halo 2 proof of the bundle.
    pub proof: Vec<u8>,
    /// RedPallas binding signature of the bundle.
    #[cfg_attr(
        all(feature = "serde-conversion", not(feature = "json-conversion")),
        serde(with = "crate::serialization::serde_bytes")
    )]
    pub binding_signature: [u8; 64],
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serialization::{PlatformDeserializableUntrusted, PlatformSerializable};

    #[test]
    fn should_round_trip_through_platform_serialization() {
        let transition = PrivateDocumentCreateFromShieldedPoolTransitionV0 {
            data_contract_id: Identifier::from([7u8; 32]),
            document_type_name: "privateContactRequest".to_string(),
            entries: vec![
                BinaryData::new(vec![1u8; 585]),
                BinaryData::new(vec![2u8; 585]),
            ],
            actions: vec![SerializedAction {
                nullifier: [1u8; 32],
                rk: [2u8; 32],
                cmx: [3u8; 32],
                encrypted_note: vec![4u8; 216],
                cv_net: [5u8; 32],
                spend_auth_sig: [6u8; 64],
            }],
            value_balance: 1000u64,
            anchor: [7u8; 32],
            proof: vec![8u8; 100],
            binding_signature: [9u8; 64],
        };
        let serialized = transition
            .serialize_to_bytes()
            .expect("expected to serialize");
        let deserialized =
            PrivateDocumentCreateFromShieldedPoolTransitionV0::deserialize_from_bytes_untrusted(
                serialized.as_slice(),
            )
            .expect("expected to deserialize");
        assert_eq!(transition, deserialized);
    }
}
