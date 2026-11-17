pub mod accessors;
pub mod methods;
mod state_transition_estimated_fee_validation;
mod state_transition_like;
mod state_transition_validation;
pub mod v0;
mod version;

use crate::state_transition::private_document_create_from_shielded_pool_transition::v0::PrivateDocumentCreateFromShieldedPoolTransitionV0;
use crate::state_transition::private_document_create_from_shielded_pool_transition::v0::PrivateDocumentCreateFromShieldedPoolTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;

pub type PrivateDocumentCreateFromShieldedPoolTransitionLatest =
    PrivateDocumentCreateFromShieldedPoolTransitionV0;

use crate::identity::state_transition::OptionallyAssetLockProved;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::From;
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize, PlatformSignable,
};
use platform_versioning::PlatformVersioned;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

/// Appends entries to a private document type's store, paid from the credit shielded pool
/// (protocol version 15).
///
/// The entries are opaque bytes of the type's `entrySize` that the chain never reads. The fee
/// is paid by an Orchard spend bundle in the credit pool whose value balance is exactly the
/// fee, so no identity signs and nothing names the sender. The target contract, type and every
/// entry are bound into the bundle's sighash, so a relay can not move the payment to other
/// entries.
#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    PlatformSerialize,
    PlatformSignable,
    PlatformVersioned,
    From,
    PartialEq,
    DecodeUntrusted,
)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
#[cfg_attr(
    all(feature = "json-conversion", feature = "serde-conversion"),
    derive(JsonConvertible)
)]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[platform_version_path_bounds(
    "dpp.state_transition_serialization_versions.private_document_create_from_shielded_pool_state_transition"
)]
pub enum PrivateDocumentCreateFromShieldedPoolTransition {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(PrivateDocumentCreateFromShieldedPoolTransitionV0),
}

impl OptionallyAssetLockProved for PrivateDocumentCreateFromShieldedPoolTransition {}

impl StateTransitionFieldTypes for PrivateDocumentCreateFromShieldedPoolTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![]
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
pub(crate) mod json_convertible_tests {
    use super::*;
    use crate::shielded::SerializedAction;
    use platform_value::{BinaryData, Identifier};

    fn fixture_action() -> SerializedAction {
        SerializedAction {
            nullifier: [0x11; 32],
            rk: [0x22; 32],
            cmx: [0x33; 32],
            encrypted_note: vec![0x44; 216],
            cv_net: [0x55; 32],
            spend_auth_sig: [0x66; 64],
        }
    }

    pub(crate) fn fixture() -> PrivateDocumentCreateFromShieldedPoolTransition {
        PrivateDocumentCreateFromShieldedPoolTransition::V0(
            PrivateDocumentCreateFromShieldedPoolTransitionV0 {
                data_contract_id: Identifier::new([0x10; 32]),
                document_type_name: "privateContactRequest".to_string(),
                entries: vec![
                    BinaryData::new(vec![0xab; 24]),
                    BinaryData::new(vec![0xcd; 24]),
                ],
                actions: vec![fixture_action()],
                value_balance: 131_000_007,
                anchor: [0x77; 32],
                proof: vec![0x88; 192],
                binding_signature: [0x99; 64],
            },
        )
    }

    #[test]
    fn should_round_trip_through_json() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        assert_eq!(json["$formatVersion"], "0");
        assert_eq!(json["documentTypeName"], "privateContactRequest");
        assert_eq!(json["valueBalance"], 131_000_007);
        // Entries are base64 strings in JSON, as every other byte field is.
        assert!(json["entries"][0].is_string(), "{json}");
        let recovered =
            PrivateDocumentCreateFromShieldedPoolTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn should_round_trip_through_a_value() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let recovered = PrivateDocumentCreateFromShieldedPoolTransition::from_object(value)
            .expect("from_object");
        assert_eq!(original, recovered);
    }
}
