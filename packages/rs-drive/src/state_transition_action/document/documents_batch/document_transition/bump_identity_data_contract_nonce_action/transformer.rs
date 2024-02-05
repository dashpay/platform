use dpp::platform_value::Identifier;

use dpp::ProtocolError;
use dpp::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
use crate::state_transition_action::document::documents_batch::document_transition::bump_identity_data_contract_nonce_action::{BumpIdentityDataContractNonceAction, BumpIdentityDataContractNonceActionV0};
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionV0};

impl BumpIdentityDataContractNonceAction {
    /// from base transition
    pub fn from_base_transition(
        value: DocumentBaseTransition,
        identity_id: Identifier,
    ) -> Result<Self, ProtocolError> {
        match value {
            DocumentBaseTransition::V0(v0) => Ok(
                BumpIdentityDataContractNonceActionV0::try_from_base_transition(v0, identity_id)?
                    .into(),
            ),
        }
    }

    /// from borrowed base transition
    pub fn from_borrowed_base_transition(
        value: &DocumentBaseTransition,
        identity_id: Identifier,
    ) -> Result<Self, ProtocolError> {
        match value {
            DocumentBaseTransition::V0(v0) => Ok(
                BumpIdentityDataContractNonceActionV0::try_from_borrowed_base_transition(
                    v0,
                    identity_id,
                )?
                .into(),
            ),
        }
    }

    /// from base transition
    pub fn from_base_transition_action(
        value: DocumentBaseTransitionAction,
        identity_id: Identifier,
    ) -> Result<Self, ProtocolError> {
        match value {
            DocumentBaseTransitionAction::V0(v0) => Ok(
                BumpIdentityDataContractNonceActionV0::try_from_base_transition_action(
                    v0,
                    identity_id,
                )?
                .into(),
            ),
        }
    }

    /// from borrowed base transition
    pub fn from_borrowed_base_transition_action(
        value: &DocumentBaseTransitionAction,
        identity_id: Identifier,
    ) -> Result<Self, ProtocolError> {
        match value {
            DocumentBaseTransitionAction::V0(v0) => Ok(
                BumpIdentityDataContractNonceActionV0::try_from_borrowed_base_transition_action(
                    v0,
                    identity_id,
                )?
                .into(),
            ),
        }
    }
}
