mod from_document;
pub mod v0_methods;

use crate::prelude::{BlockHeight, CoreBlockHeight, Revision, TimestampMillis};
use bincode::{Decode, Encode};
use derive_more::Display;

use platform_value::{Identifier, Value};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

use crate::block::block_info::BlockInfo;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::{Document, DocumentV0};
use crate::{document, ProtocolError};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

pub use super::super::document_base_transition::IDENTIFIER_FIELDS;
use crate::state_transition::documents_batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;

mod property_names {
    pub const REVISION: &str = "$revision";
}

#[derive(Debug, Clone, Default, Encode, Decode, PartialEq, Display)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[display(fmt = "Base: {}, Revision: {}, Data: {:?}", "base", "revision", "data")]
pub struct DocumentReplaceTransitionV0 {
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(flatten))]
    pub base: DocumentBaseTransition,
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "$revision")
    )]
    pub revision: Revision,
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(flatten))]
    pub data: BTreeMap<String, Value>,
}

/// document from replace transition v0
pub trait DocumentFromReplaceTransitionV0 {
    /// Attempts to create a new `Document` from the given `DocumentReplaceTransitionV0` reference. This operation is typically used to replace or update an existing document with new information.
    ///
    /// # Arguments
    ///
    /// * `value` - A reference to the `DocumentReplaceTransitionV0` containing the new information for the document.
    /// * `owner_id` - The `Identifier` of the document's owner.
    /// * `created_at` - An optional timestamp representing when the original document was created. This is preserved during replacement.
    /// * `created_at_block_height` - An optional height of the block at which the original document was created. This is preserved during replacement.
    /// * `created_at_core_block_height` - An optional core block height at which the original document was created. This is preserved during replacement.
    /// * `block_info` - Information about the current block at the time of this replace transition.
    /// * `document_type` - A reference to the `DocumentTypeRef` indicating the type of the document being replaced.
    /// * `platform_version` - A reference to the `PlatformVersion` indicating the version of the platform under which the document is being replaced.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - On successful replacement, returns a new `Document` object populated with the provided data. Returns a `ProtocolError` if the replacement fails due to validation errors or other issues.
    ///
    /// # Errors
    ///
    /// This function may return a `ProtocolError` if validation fails, required fields are missing, or if there are mismatches between field types and the schema defined in the data contract.
    fn try_from_replace_transition_v0(
        value: &DocumentReplaceTransitionV0,
        owner_id: Identifier,
        created_at: Option<TimestampMillis>,
        created_at_block_height: Option<BlockHeight>,
        created_at_core_block_height: Option<CoreBlockHeight>,
        transferred_at: Option<TimestampMillis>,
        transferred_at_block_height: Option<BlockHeight>,
        transferred_at_core_block_height: Option<CoreBlockHeight>,
        block_info: &BlockInfo,
        document_type: &DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    /// Attempts to create a new `Document` from the given `DocumentReplaceTransitionV0` instance. This function is similar to `try_from_replace_transition_v0` but consumes the `DocumentReplaceTransitionV0` instance, making it suitable for scenarios where the transition is owned and should not be reused after document creation.
    ///
    /// # Arguments
    ///
    /// * `value` - An owned `DocumentReplaceTransitionV0` instance containing the new information for the document.
    /// * `owner_id` - The `Identifier` of the document's owner.
    /// * `created_at` - An optional timestamp representing when the original document was created. This is preserved during replacement.
    /// * `created_at_block_height` - An optional height of the block at which the original document was created. This is preserved during replacement.
    /// * `created_at_core_block_height` - An optional core block height at which the original document was created. This is preserved during replacement.
    /// * `block_info` - Information about the current block at the time of this replace transition.
    /// * `document_type` - A reference to the `DocumentTypeRef` indicating the type of the document being replaced.
    /// * `platform_version` - A reference to the `PlatformVersion` indicating the version of the platform under which the document is being replaced.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - On successful replacement, returns a new `Document` object populated with the provided data. Returns a `ProtocolError` if the replacement fails due to validation errors or other issues.
    ///
    /// # Errors
    ///
    /// This function may return a `ProtocolError` for the same reasons as `try_from_replace_transition_v0`, including validation failures, missing required fields, or schema mismatches.
    fn try_from_owned_replace_transition_v0(
        value: DocumentReplaceTransitionV0,
        owner_id: Identifier,
        created_at: Option<TimestampMillis>,
        created_at_block_height: Option<BlockHeight>,
        created_at_core_block_height: Option<CoreBlockHeight>,
        transferred_at: Option<TimestampMillis>,
        transferred_at_block_height: Option<BlockHeight>,
        transferred_at_core_block_height: Option<CoreBlockHeight>,
        block_info: &BlockInfo,
        document_type: &DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

impl DocumentFromReplaceTransitionV0 for Document {
    fn try_from_replace_transition_v0(
        value: &DocumentReplaceTransitionV0,
        owner_id: Identifier,
        created_at: Option<TimestampMillis>,
        created_at_block_height: Option<BlockHeight>,
        created_at_core_block_height: Option<CoreBlockHeight>,
        transferred_at: Option<TimestampMillis>,
        transferred_at_block_height: Option<BlockHeight>,
        transferred_at_core_block_height: Option<CoreBlockHeight>,
        block_info: &BlockInfo,
        document_type: &DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let DocumentReplaceTransitionV0 {
            base,
            revision,
            data,
        } = value;

        let id = base.id();

        let requires_updated_at = document_type
            .required_fields()
            .contains(document::property_names::UPDATED_AT);

        let requires_updated_at_block_height = document_type
            .required_fields()
            .contains(document::property_names::UPDATED_AT_BLOCK_HEIGHT);

        let requires_updated_at_core_block_height = document_type
            .required_fields()
            .contains(document::property_names::UPDATED_AT_CORE_BLOCK_HEIGHT);

        let updated_at = if requires_updated_at {
            Some(block_info.time_ms)
        } else {
            None
        };

        let updated_at_block_height = if requires_updated_at_block_height {
            Some(block_info.height)
        } else {
            None
        };

        let updated_at_core_block_height = if requires_updated_at_core_block_height {
            Some(block_info.core_height)
        } else {
            None
        };

        match platform_version
            .dpp
            .document_versions
            .document_structure_version
        {
            0 => Ok(DocumentV0 {
                id,
                owner_id,
                properties: data.clone(),
                revision: Some(*revision),
                created_at,
                updated_at,
                transferred_at,
                created_at_block_height,
                updated_at_block_height,
                transferred_at_block_height,
                created_at_core_block_height,
                updated_at_core_block_height,
                transferred_at_core_block_height,
            }
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::try_from_replace_transition".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn try_from_owned_replace_transition_v0(
        value: DocumentReplaceTransitionV0,
        owner_id: Identifier,
        created_at: Option<TimestampMillis>,
        created_at_block_height: Option<BlockHeight>,
        created_at_core_block_height: Option<CoreBlockHeight>,
        transferred_at: Option<TimestampMillis>,
        transferred_at_block_height: Option<BlockHeight>,
        transferred_at_core_block_height: Option<CoreBlockHeight>,
        block_info: &BlockInfo,
        document_type: &DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let DocumentReplaceTransitionV0 {
            base,
            revision,
            data,
        } = value;

        let id = base.id();

        let requires_updated_at = document_type
            .required_fields()
            .contains(document::property_names::UPDATED_AT);

        let requires_updated_at_block_height = document_type
            .required_fields()
            .contains(document::property_names::UPDATED_AT_BLOCK_HEIGHT);

        let requires_updated_at_core_block_height = document_type
            .required_fields()
            .contains(document::property_names::UPDATED_AT_CORE_BLOCK_HEIGHT);

        let updated_at = if requires_updated_at {
            Some(block_info.time_ms)
        } else {
            None
        };

        let updated_at_block_height = if requires_updated_at_block_height {
            Some(block_info.height)
        } else {
            None
        };

        let updated_at_core_block_height = if requires_updated_at_core_block_height {
            Some(block_info.core_height)
        } else {
            None
        };
        match platform_version
            .dpp
            .document_versions
            .document_structure_version
        {
            0 => Ok(DocumentV0 {
                id,
                owner_id,
                properties: data,
                revision: Some(revision),
                created_at,
                updated_at,
                transferred_at,
                created_at_block_height,
                updated_at_block_height,
                transferred_at_block_height,
                created_at_core_block_height,
                updated_at_core_block_height,
                transferred_at_core_block_height,
            }
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::try_from_replace_transition".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
