mod from_document;
pub mod v0;
pub mod v0_methods;

use crate::block::block_info::BlockInfo;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::Document;
use crate::prelude::{BlockHeight, CoreBlockHeight, TimestampMillis};
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::{Display, From};
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
pub use v0::*;

#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
pub enum DocumentReplaceTransition {
    #[display("V0({})", "_0")]
    V0(DocumentReplaceTransitionV0),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for DocumentReplaceTransition {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for DocumentReplaceTransition {}

/// document from replace transition
pub trait DocumentFromReplaceTransition {
    /// Attempts to create a new `Document` from the given `DocumentReplaceTransition` reference, incorporating `owner_id`, creation metadata, and additional blockchain-related information.
    ///
    /// This method is designed to replace an existing document with new information, while also preserving and incorporating specific metadata about the document's creation and update history.
    ///
    /// # Arguments
    ///
    /// * `document_replace_transition_action` - A reference to the `DocumentReplaceTransition` containing the new information for the document being replaced.
    /// * `owner_id` - The `Identifier` of the document's owner.
    /// * `created_at` - An optional timestamp indicating when the original document was created.
    /// * `created_at_block_height` - An optional block height indicating when the original document was created.
    /// * `created_at_core_block_height` - An optional core block height indicating when the original document was created.
    /// * `transferred_at` - An optional timestamp indicating when the document was last transferred.
    /// * `transferred_at_block_height` - An optional block height indicating when the document was last transferred.
    /// * `transferred_at_core_block_height` - An optional core block height indicating when the document was last transferred.
    /// * `creator_id` - An optional `Identifier` of the document's original creator.
    /// * `block_info` - Current block information used for updating document metadata.
    /// * `document_type` - Reference to the document type to ensure compatibility and proper validation.
    /// * `platform_version` - Reference to the current platform version to check for compatibility and apply version-specific logic.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - On successful document replacement, returns a new `Document` object populated with the provided data and metadata. On failure, returns a `ProtocolError` detailing the issue.
    ///
    /// # Errors
    ///
    /// This function may return `ProtocolError` if there are validation errors related to document data, missing required metadata, or incompatibilities with the current platform version.
    #[allow(clippy::too_many_arguments)]
    fn try_from_replace_transition(
        document_replace_transition_action: &DocumentReplaceTransition,
        owner_id: Identifier,
        created_at: Option<TimestampMillis>,
        created_at_block_height: Option<BlockHeight>,
        created_at_core_block_height: Option<CoreBlockHeight>,
        transferred_at: Option<TimestampMillis>,
        transferred_at_block_height: Option<BlockHeight>,
        transferred_at_core_block_height: Option<CoreBlockHeight>,
        creator_id: Option<Identifier>,
        block_info: &BlockInfo,
        document_type: &DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;

    /// Attempts to create a new `Document` from the given `DocumentReplaceTransition` instance, incorporating `owner_id`, creation metadata, and additional blockchain-related information.
    ///
    /// This method functions similarly to `try_from_replace_transition`, but it consumes the `DocumentReplaceTransition` instance, making it suitable for use cases where the transition is not needed after document creation.
    ///
    /// # Arguments
    ///
    /// * `document_replace_transition_action` - An owned `DocumentReplaceTransition` instance containing the new information for the document being replaced.
    /// * `owner_id` - The `Identifier` of the document's owner.
    /// * `created_at` - An optional timestamp indicating when the original document was created.
    /// * `created_at_block_height` - An optional block height indicating when the original document was created.
    /// * `created_at_core_block_height` - An optional core block height indicating when the original document was created.
    /// * `transferred_at` - An optional timestamp indicating when the document was last transferred.
    /// * `transferred_at_block_height` - An optional block height indicating when the document was last transferred.
    /// * `transferred_at_core_block_height` - An optional core block height indicating when the document was last transferred.
    /// * `creator_id` - An optional `Identifier` of the document's original creator.
    /// * `block_info` - Current block information used for updating document metadata.
    /// * `document_type` - Reference to the document type to ensure compatibility and proper validation.
    /// * `platform_version` - Reference to the current platform version to check for compatibility and apply version-specific logic.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - On successful document replacement, returns a new `Document` object. On failure, returns a `ProtocolError` detailing the issue.
    ///
    /// # Errors
    ///
    /// This function may return `ProtocolError` for the same reasons as `try_from_replace_transition`, including validation failures, missing metadata, or platform incompatibilities.
    #[allow(clippy::too_many_arguments)]
    fn try_from_owned_replace_transition(
        document_replace_transition_action: DocumentReplaceTransition,
        owner_id: Identifier,
        created_at: Option<TimestampMillis>,
        created_at_block_height: Option<BlockHeight>,
        created_at_core_block_height: Option<CoreBlockHeight>,
        transferred_at: Option<TimestampMillis>,
        transferred_at_block_height: Option<BlockHeight>,
        transferred_at_core_block_height: Option<CoreBlockHeight>,
        creator_id: Option<Identifier>,
        block_info: &BlockInfo,
        document_type: &DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

impl DocumentFromReplaceTransition for Document {
    fn try_from_replace_transition(
        document_replace_transition: &DocumentReplaceTransition,
        owner_id: Identifier,
        created_at: Option<TimestampMillis>,
        created_at_block_height: Option<BlockHeight>,
        created_at_core_block_height: Option<CoreBlockHeight>,
        transferred_at: Option<TimestampMillis>,
        transferred_at_block_height: Option<BlockHeight>,
        transferred_at_core_block_height: Option<CoreBlockHeight>,
        creator_id: Option<Identifier>,
        block_info: &BlockInfo,
        document_type: &DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match document_replace_transition {
            DocumentReplaceTransition::V0(v0) => Self::try_from_replace_transition_v0(
                v0,
                owner_id,
                created_at,
                created_at_block_height,
                created_at_core_block_height,
                transferred_at,
                transferred_at_block_height,
                transferred_at_core_block_height,
                creator_id,
                block_info,
                document_type,
                platform_version,
            ),
        }
    }

    fn try_from_owned_replace_transition(
        document_replace_transition: DocumentReplaceTransition,
        owner_id: Identifier,
        created_at: Option<TimestampMillis>,
        created_at_block_height: Option<BlockHeight>,
        created_at_core_block_height: Option<CoreBlockHeight>,
        transferred_at: Option<TimestampMillis>,
        transferred_at_block_height: Option<BlockHeight>,
        transferred_at_core_block_height: Option<CoreBlockHeight>,
        creator_id: Option<Identifier>,
        block_info: &BlockInfo,
        document_type: &DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match document_replace_transition {
            DocumentReplaceTransition::V0(v0) => Self::try_from_owned_replace_transition_v0(
                v0,
                owner_id,
                created_at,
                created_at_block_height,
                created_at_core_block_height,
                transferred_at,
                transferred_at_block_height,
                transferred_at_core_block_height,
                creator_id,
                block_info,
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
    use crate::state_transition::batch_transition::document_replace_transition::v0::DocumentReplaceTransitionV0;
    use platform_value::{Identifier, Value};
    use std::collections::BTreeMap;

    fn base_fixture() -> DocumentBaseTransition {
        DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: Identifier::new([0xc1; 32]),
            identity_contract_nonce: 11,
            document_type_name: "post".to_string(),
            data_contract_id: Identifier::new([0xd2; 32]),
        })
    }

    fn data_fixture() -> BTreeMap<String, Value> {
        let mut data = BTreeMap::new();
        data.insert("name".to_string(), Value::Text("alice".to_string()));
        data
    }

    fn fixture() -> DocumentReplaceTransition {
        DocumentReplaceTransition::V0(DocumentReplaceTransitionV0 {
            base: base_fixture(),
            revision: 5,
            data: data_fixture(),
        })
    }

    #[test]
    fn json_round_trip() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = JsonConvertible::to_json(&original).expect("to_json");
        let recovered = <DocumentReplaceTransition as JsonConvertible>::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = ValueConvertible::to_object(&original).expect("to_object");
        let recovered = <DocumentReplaceTransition as ValueConvertible>::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
