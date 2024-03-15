mod from_document;
pub mod v0;
pub mod v0_methods;

use crate::document::Document;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::{Display, From};
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};
pub use v0::*;

#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize)
)]
pub enum DocumentReplaceTransition {
    #[display(fmt = "V0({})", "_0")]
    V0(DocumentReplaceTransitionV0),
}

/// document from replace transition
pub trait DocumentFromReplaceTransition {
    /// Attempts to create a new `Document` from the given `DocumentReplaceTransition` reference, `owner_id`, and additional metadata.
    ///
    /// # Arguments
    ///
    /// * `document_replace_transition_action` - A reference to the `DocumentReplaceTransition` containing information about the document being replaced.
    /// * `owner_id` - The `Identifier` of the document's owner.
    /// * `created_at` - An optional timestamp representing when the document was created.
    /// * `block_time` - The timestamp representing the blockchain time at which the document was updated.
    /// * `requires_updated_at` - A boolean indicating if an `updated_at` timestamp is required for the document.
    /// * `platform_version` - A reference to the `PlatformVersion` indicating the version of the platform for compatibility.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - A new `Document` object if successful, otherwise a `ProtocolError`.
    fn try_from_replace_transition(
        document_replace_transition_action: &DocumentReplaceTransition,
        owner_id: Identifier,
        created_at: Option<u64>,
        block_time: u64,
        requires_updated_at: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;

    /// Attempts to create a new `Document` from the given `DocumentReplaceTransition` instance, `owner_id`, and additional metadata.
    ///
    /// # Arguments
    ///
    /// * `document_replace_transition_action` - A `DocumentReplaceTransition` instance containing information about the document being replaced.
    /// * `owner_id` - The `Identifier` of the document's owner.
    /// * `created_at` - An optional timestamp representing when the document was created.
    /// * `block_time` - The timestamp representing the blockchain time at which the document was updated.
    /// * `requires_updated_at` - A boolean indicating if an `updated_at` timestamp is required for the document.
    /// * `platform_version` - A reference to the `PlatformVersion` indicating the version of the platform for compatibility.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - A new `Document` object if successful, otherwise a `ProtocolError`.
    fn try_from_owned_replace_transition(
        document_replace_transition_action: DocumentReplaceTransition,
        owner_id: Identifier,
        created_at: Option<u64>,
        block_time: u64,
        requires_updated_at: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

impl DocumentFromReplaceTransition for Document {
    fn try_from_replace_transition(
        document_replace_transition: &DocumentReplaceTransition,
        owner_id: Identifier,
        created_at: Option<u64>,
        block_time: u64,
        requires_updated_at: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match document_replace_transition {
            DocumentReplaceTransition::V0(v0) => Self::try_from_replace_transition_v0(
                v0,
                owner_id,
                created_at,
                block_time,
                requires_updated_at,
                platform_version,
            ),
        }
    }

    fn try_from_owned_replace_transition(
        document_replace_transition: DocumentReplaceTransition,
        owner_id: Identifier,
        created_at: Option<u64>,
        block_time: u64,
        requires_updated_at: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match document_replace_transition {
            DocumentReplaceTransition::V0(v0) => Self::try_from_owned_replace_transition_v0(
                v0,
                owner_id,
                created_at,
                block_time,
                requires_updated_at,
                platform_version,
            ),
        }
    }
}
