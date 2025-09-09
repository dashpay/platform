use dpp::platform_value::Identifier;
use dpp::prelude::UserFeeIncrease;
use dpp::ProtocolError;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::token_transition::TokenTransitionV0Methods;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use dpp::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use crate::state_transition_action::contract::data_contract_update::DataContractUpdateTransitionAction;
use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::{BumpIdentityDataContractNonceAction, BumpIdentityDataContractNonceActionV0};

impl BumpIdentityDataContractNonceAction {
    /// from borrowed base transition
    pub fn from_batched_transition_ref(
        value: BatchedTransitionRef,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        match value {
            BatchedTransitionRef::Document(document) => {
                Self::from_borrowed_document_base_transition(
                    document.base(),
                    identity_id,
                    user_fee_increase,
                )
            }
            BatchedTransitionRef::Token(token) => Self::from_borrowed_token_base_transition(
                token.base(),
                identity_id,
                user_fee_increase,
            ),
        }
    }

    /// helper method
    pub fn try_from_batched_transition_action(
        value: BatchedTransitionAction,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Result<Self, Error> {
        match value {
            BatchedTransitionAction::DocumentAction(document) => {
                Ok(Self::from_document_base_transition_action(
                    document.base_owned(),
                    identity_id,
                    user_fee_increase,
                ))
            }
            BatchedTransitionAction::TokenAction(token) => Ok(Self::from_token_base_transition_action(
                token.base_owned(),
                identity_id,
                user_fee_increase,
            )),
            BatchedTransitionAction::BumpIdentityDataContractNonce(_) => {
                Err(Error::Protocol(Box::new(
                    ProtocolError::CorruptedCodeExecution(
                        "we should never be trying to convert from a BumpIdentityDataContractNonce to a BumpIdentityDataContractNonceAction".to_string(),
                    ),
                )))
            }
        }
    }

    /// helper method
    pub fn try_from_borrowed_batched_transition_action(
        value: &BatchedTransitionAction,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Result<Self, Error> {
        match value {
            BatchedTransitionAction::DocumentAction(document) => {
                Ok(Self::from_borrowed_document_base_transition_action(
                    document.base(),
                    identity_id,
                    user_fee_increase,
                ))
            }
            BatchedTransitionAction::TokenAction(token) => Ok(Self::from_borrowed_token_base_transition_action(
                token.base(),
                identity_id,
                user_fee_increase,
            )),
            BatchedTransitionAction::BumpIdentityDataContractNonce(_) => {
                Err(Error::Protocol(Box::new(
                    ProtocolError::CorruptedCodeExecution(
                        "we should never be trying to convert from a BumpIdentityDataContractNonce to a BumpIdentityDataContractNonceAction".to_string(),
                    ),
                )))
            }
        }
    }

    /// from base transition
    pub fn from_document_base_transition(
        value: DocumentBaseTransition,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        BumpIdentityDataContractNonceActionV0::from_document_base_transition(
            value,
            identity_id,
            user_fee_increase,
        )
        .into()
    }

    /// from borrowed base transition
    pub fn from_borrowed_document_base_transition(
        value: &DocumentBaseTransition,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        BumpIdentityDataContractNonceActionV0::from_borrowed_document_base_transition(
            value,
            identity_id,
            user_fee_increase,
        )
        .into()
    }

    /// from base transition
    pub fn from_document_base_transition_action(
        value: DocumentBaseTransitionAction,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        BumpIdentityDataContractNonceActionV0::from_document_base_transition_action(
            value,
            identity_id,
            user_fee_increase,
        )
        .into()
    }

    /// from borrowed base transition
    pub fn from_borrowed_document_base_transition_action(
        value: &DocumentBaseTransitionAction,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        BumpIdentityDataContractNonceActionV0::from_borrowed_document_base_transition_action(
            value,
            identity_id,
            user_fee_increase,
        )
        .into()
    }

    /// from base transition
    pub fn from_token_base_transition(
        value: TokenBaseTransition,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        BumpIdentityDataContractNonceActionV0::from_token_base_transition(
            value,
            identity_id,
            user_fee_increase,
        )
        .into()
    }

    /// from borrowed base transition
    pub fn from_borrowed_token_base_transition(
        value: &TokenBaseTransition,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        BumpIdentityDataContractNonceActionV0::from_borrowed_token_base_transition(
            value,
            identity_id,
            user_fee_increase,
        )
        .into()
    }

    /// from base transition
    pub fn from_token_base_transition_action(
        value: TokenBaseTransitionAction,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        BumpIdentityDataContractNonceActionV0::from_token_base_transition_action(
            value,
            identity_id,
            user_fee_increase,
        )
        .into()
    }

    /// from borrowed base transition
    pub fn from_borrowed_token_base_transition_action(
        value: &TokenBaseTransitionAction,
        identity_id: Identifier,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        BumpIdentityDataContractNonceActionV0::from_borrowed_token_base_transition_action(
            value,
            identity_id,
            user_fee_increase,
        )
        .into()
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
