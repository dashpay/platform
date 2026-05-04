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
    use platform_value::{Identifier, Value};
    use std::collections::BTreeMap;

    /// Non-default values per field so a per-property assertion would catch
    /// any silent zero-out / flip on round-trip.
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

    fn assert_v0_fields(t: &DocumentCreateTransition) {
        let DocumentCreateTransition::V0(rec) = t;
        let DocumentBaseTransition::V0(base) = &rec.base else { panic!("expected base V0"); };
        assert_eq!(base.id, Identifier::new([0xc1; 32]), "base.id");
        assert_eq!(base.identity_contract_nonce, 11, "base.identity_contract_nonce");
        assert_eq!(base.document_type_name, "post", "base.document_type_name");
        assert_eq!(base.data_contract_id, Identifier::new([0xd2; 32]), "base.data_contract_id");
        assert_eq!(rec.entropy, [0xab; 32], "entropy");
        assert_eq!(
            rec.data.get("name"),
            Some(&Value::Text("alice".to_string())),
            "data.name"
        );
        assert_eq!(
            rec.prefunded_voting_balance,
            Some(("uniqueName".to_string(), 50_000)),
            "prefunded_voting_balance"
        );
    }

    #[test]
    fn json_round_trip_with_per_property_assertions() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = JsonConvertible::to_json(&original).expect("to_json");
        let recovered = <DocumentCreateTransition as JsonConvertible>::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
        assert_v0_fields(&recovered);
    }

    #[test]
    fn value_round_trip_with_per_property_assertions() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = ValueConvertible::to_object(&original).expect("to_object");
        let recovered = <DocumentCreateTransition as ValueConvertible>::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
        assert_v0_fields(&recovered);
    }
}
