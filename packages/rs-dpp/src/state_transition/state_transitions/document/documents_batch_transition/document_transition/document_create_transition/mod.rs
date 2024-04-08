mod convertible;
pub mod from_document;
pub mod v0;
mod v0_methods;

use crate::block::block_info::BlockInfo;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::Document;
use crate::state_transition::documents_batch_transition::document_create_transition::v0::DocumentFromCreateTransitionV0;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::{Display, From};
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};
pub use v0::DocumentCreateTransitionV0;

#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize)
)]
pub enum DocumentCreateTransition {
    #[display(fmt = "V0({})", "_0")]
    V0(DocumentCreateTransitionV0),
}

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
                document_type,
                platform_version,
            ),
        }
    }

    fn try_from_owned_create_transition(
        document_create_transition: DocumentCreateTransition,
        owner_id: Identifier,
        block_info: &BlockInfo,
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
                document_type,
                platform_version,
            ),
        }
    }
}
