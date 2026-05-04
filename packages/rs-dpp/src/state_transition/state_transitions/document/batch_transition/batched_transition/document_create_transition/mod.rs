mod convertible;
pub mod from_document;
pub mod v0;
mod v0_methods;

use crate::block::block_info::BlockInfo;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::Document;
use crate::prelude::DataContract;
use crate::state_transition::batch_transition::document_create_transition::v0::DocumentFromCreateTransitionV0;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::{Display, From};
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
pub use v0::DocumentCreateTransitionV0;

#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
pub enum DocumentCreateTransition {
    #[display("V0({})", "_0")]
    V0(DocumentCreateTransitionV0),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for DocumentCreateTransition {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for DocumentCreateTransition {}

impl Default for DocumentCreateTransition {
    fn default() -> Self {
        DocumentCreateTransition::V0(DocumentCreateTransitionV0::default()) // since only v0
    }
}

/// document from create transition
pub trait DocumentFromCreateTransition {
    /// Attempts to create a new `Document` from the given `DocumentCreateTransition` reference, `owner_id`, and additional metadata.
    ///
    /// # Arguments
    ///
    /// * `document_create_transition` - A reference to the `DocumentCreateTransition` containing information about the document being created.
    /// * `owner_id` - The `Identifier` of the document's owner.
    /// * `block_info` - The block info containing information about the current block such as block time, block height and core block height.
    /// * `document_type` - A reference to the `DocumentTypeRef` associated with this document, defining its structure and rules.
    /// * `platform_version` - A reference to the `PlatformVersion` indicating the version of the platform for compatibility.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - A new `Document` object if successful, otherwise a `ProtocolError`.
    fn try_from_create_transition(
        document_create_transition: &DocumentCreateTransition,
        owner_id: Identifier,
        block_info: &BlockInfo,
        contract: &DataContract,
        document_type: &DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;

    /// Attempts to create a new `Document` from the given `DocumentCreateTransition` instance, `owner_id`, and additional metadata.
    ///
    /// # Arguments
    ///
    /// * `document_create_transition` - A `DocumentCreateTransition` instance containing information about the document being created.
    /// * `owner_id` - The `Identifier` of the document's owner.
    /// * `block_info` - The block info containing information about the current block such as block time, block height and core block height.
    /// * `document_type` - A reference to the `DocumentTypeRef` associated with this document, defining its structure and rules.
    /// * `platform_version` - A reference to the `PlatformVersion` indicating the version of the platform for compatibility.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - A new `Document` object if successful, otherwise a `ProtocolError`.
    fn try_from_owned_create_transition(
        document_create_transition: DocumentCreateTransition,
        owner_id: Identifier,
        block_info: &BlockInfo,
        contract: &DataContract,
        document_type: &DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

impl DocumentFromCreateTransition for Document {
    fn try_from_create_transition(
        document_create_transition: &DocumentCreateTransition,
        owner_id: Identifier,
        block_info: &BlockInfo,
        contract: &DataContract,
        document_type: &DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        match document_create_transition {
            DocumentCreateTransition::V0(v0) => Self::try_from_create_transition_v0(
                v0,
                owner_id,
                block_info,
                contract,
                document_type,
                platform_version,
            ),
        }
    }

    fn try_from_owned_create_transition(
        document_create_transition: DocumentCreateTransition,
        owner_id: Identifier,
        block_info: &BlockInfo,
        contract: &DataContract,
        document_type: &DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        match document_create_transition {
            DocumentCreateTransition::V0(v0) => Self::try_from_owned_create_transition_v0(
                v0,
                owner_id,
                block_info,
                contract,
                document_type,
                platform_version,
            ),
        }
    }
}

#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
mod json_convertible_tests {
    use super::*;
    use crate::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
    use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
    use crate::state_transition::batch_transition::document_create_transition::v0::DocumentCreateTransitionV0;
    use platform_value::{platform_value, Identifier, Value};
    use serde_json::json;
    use std::collections::BTreeMap;

    /// Non-default values per field so the wire-shape assertion catches any
    /// silent zero-out / flip on round-trip.
    fn fixture() -> DocumentCreateTransition {
        let mut data = BTreeMap::new();
        data.insert("name".to_string(), Value::Text("alice".to_string()));
        DocumentCreateTransition::V0(DocumentCreateTransitionV0 {
            base: DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
                id: Identifier::new([0xc1; 32]),
                identity_contract_nonce: 11,
                document_type_name: "post".to_string(),
                data_contract_id: Identifier::new([0xd2; 32]),
            }),
            entropy: [0xab; 32],
            data,
            prefunded_voting_balance: Some(("uniqueName".to_string(), 50_000)),
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        let entropy_vec: Vec<u8> = vec![0xab; 32];
        // Externally-tagged enum: outer `V0` wraps the variant; the inner
        // `base` field is itself an enum (`DocumentBaseTransition::V0`),
        // so it appears as `{"V0": {...base fields...}}` flattened into the
        // outer map. `$entropy` is a `[u8; 32]` -> JSON renders it as an
        // array of numbers (no base64 envelope). `$identityContractNonce`
        // is `u64`; JSON has only one number type, so the size is erased.
        // `data` is `#[serde(flatten)]` -> the map's keys become top-level.
        // `$prefundedVotingBalance` is `Option<(String, u64)>` and
        // serializes as a 2-element JSON array.
        assert_eq!(
            json,
            json!({
                "V0": {
                    "V0": {
                        "$id": Identifier::new([0xc1; 32]),
                        "$identityContractNonce": 11,
                        "$type": "post",
                        "$dataContractId": Identifier::new([0xd2; 32]),
                    },
                    "$entropy": entropy_vec,
                    "name": "alice",
                    "$prefundedVotingBalance": ["uniqueName", 50_000],
                }
            })
        );
        let recovered = DocumentCreateTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let entropy: [u8; 32] = [0xab; 32];
        // `11u64`: `IdentityNonce` is a `u64` alias; explicit suffix locks in
        // the sized `Value::U64`. `[u8; 32]`: each element preserved as
        // `Value::U8` via the platform_value! array path. `50_000u64`:
        // `Credits` is a `u64` alias. `Identifier`s interpolate via
        // `Serialize` -> `Value::Identifier`.
        assert_eq!(
            value,
            platform_value!({
                "V0": {
                    "V0": {
                        "$id": Identifier::new([0xc1; 32]),
                        "$identityContractNonce": 11u64,
                        "$type": "post",
                        "$dataContractId": Identifier::new([0xd2; 32]),
                    },
                    "$entropy": entropy,
                    "name": "alice",
                    "$prefundedVotingBalance": ["uniqueName", 50_000u64],
                }
            })
        );
        let recovered = DocumentCreateTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
