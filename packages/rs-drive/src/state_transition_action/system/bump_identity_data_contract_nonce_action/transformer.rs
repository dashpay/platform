use dpp::platform_value::Identifier;
use dpp::prelude::UserFeeIncrease;

use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
use crate::state_transition_action::contract::data_contract_update::DataContractUpdateTransitionAction;
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::{BumpIdentityDataContractNonceAction, BumpIdentityDataContractNonceActionV0};

impl BumpIdentityDataContractNonceAction {
    /// from base transition
    pub fn from_document_base_transition(
        value: DocumentBaseTransition,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        match value {
            DocumentBaseTransition::V0(v0) => {
                BumpIdentityDataContractNonceActionV0::from_base_transition(
                    v0,
                    identity_id,
                    user_fee_increase,
                )
                .into()
            }
        }
    }

    /// from borrowed base transition
    pub fn from_borrowed_document_base_transition(
        value: &DocumentBaseTransition,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        match value {
            DocumentBaseTransition::V0(v0) => {
                BumpIdentityDataContractNonceActionV0::from_borrowed_base_transition(
                    v0,
                    identity_id,
                    user_fee_increase,
                )
                .into()
            }
        }
    }

    /// from base transition
    pub fn from_document_base_transition_action(
        value: DocumentBaseTransitionAction,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        match value {
            DocumentBaseTransitionAction::V0(v0) => {
                BumpIdentityDataContractNonceActionV0::from_base_transition_action(
                    v0,
                    identity_id,
                    user_fee_increase,
                )
                .into()
            }
        }
    }

    /// from borrowed base transition
    pub fn from_borrowed_document_base_transition_action(
        value: &DocumentBaseTransitionAction,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        match value {
            DocumentBaseTransitionAction::V0(v0) => {
                BumpIdentityDataContractNonceActionV0::from_borrowed_base_transition_action(
                    v0,
                    identity_id,
                    user_fee_increase,
                )
                .into()
            }
        }
    }

    /// from data contract update
    pub fn from_data_contract_update_transition(value: DataContractUpdateTransition) -> Self {
        match value {
            DataContractUpdateTransition::V0(v0) => {
                BumpIdentityDataContractNonceActionV0::from_data_contract_update(v0).into()
            }
        }
    }

    /// from borrowed data contract update
    pub fn from_borrowed_data_contract_update_transition(
        value: &DataContractUpdateTransition,
    ) -> Self {
        match value {
            DataContractUpdateTransition::V0(v0) => {
                BumpIdentityDataContractNonceActionV0::from_borrowed_data_contract_update(v0).into()
            }
        }
    }

    /// from data contract update action
    pub fn from_data_contract_update_transition_action(
        value: DataContractUpdateTransitionAction,
    ) -> Self {
        match value {
            DataContractUpdateTransitionAction::V0(v0) => {
                BumpIdentityDataContractNonceActionV0::from_data_contract_update_action(v0).into()
            }
        }
    }

    /// from borrowed data contract update action
    pub fn from_borrowed_data_contract_update_transition_action(
        value: &DataContractUpdateTransitionAction,
    ) -> Self {
        match value {
            DataContractUpdateTransitionAction::V0(v0) => {
                BumpIdentityDataContractNonceActionV0::from_borrowed_data_contract_update_action(v0)
                    .into()
            }
        }
    }
}
