//! Generation 1 of the create-action → `Document` conversion: generation 0
//! plus the contract-version stamp. The built document records the version
//! of the contract it was validated against, which selects each
//! `requiredSince` property's byte layout in document serialization format
//! 3. This generation must only be selected by platform versions whose
//! document serialization format writes the stamp (format 3, protocol
//! v14+); the version table pairs the two.

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::{Document, DocumentV0Setters};
use dpp::platform_value::Identifier;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;

use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionAccessorsV0};
use super::{DocumentCreateTransitionActionV0, DocumentFromCreateTransitionActionV0};

/// documents from create transition v1
pub trait DocumentFromCreateTransitionActionV1 {
    /// Attempts to create a new `Document` from the given `DocumentCreateTransitionActionV0`
    /// instance and `owner_id`, stamped with the contract version the
    /// document was created against.
    fn try_from_owned_create_transition_action_v1(
        v0: DocumentCreateTransitionActionV0,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    /// Attempts to create a new `Document` from the given `DocumentCreateTransitionActionV0`
    /// reference and `owner_id`, stamped with the contract version the
    /// document was created against.
    fn try_from_create_transition_action_v1(
        v0: &DocumentCreateTransitionActionV0,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

fn action_contract_version(base: &DocumentBaseTransitionAction) -> u32 {
    base.data_contract_fetch_info_ref().contract.version()
}

impl DocumentFromCreateTransitionActionV1 for Document {
    fn try_from_owned_create_transition_action_v1(
        v0: DocumentCreateTransitionActionV0,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let contract_version = action_contract_version(&v0.base);
        let mut document =
            Self::try_from_owned_create_transition_action_v0(v0, owner_id, platform_version)?;
        document.set_contract_version(Some(contract_version));
        Ok(document)
    }

    fn try_from_create_transition_action_v1(
        v0: &DocumentCreateTransitionActionV0,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let contract_version = action_contract_version(&v0.base);
        let mut document =
            Self::try_from_create_transition_action_v0(v0, owner_id, platform_version)?;
        document.set_contract_version(Some(contract_version));
        Ok(document)
    }
}
