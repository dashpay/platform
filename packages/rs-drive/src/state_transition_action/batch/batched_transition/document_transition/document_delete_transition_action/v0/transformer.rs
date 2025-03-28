use dpp::platform_value::Identifier;
use std::sync::Arc;
use dpp::data_contract::document_type::accessors::DocumentTypeV1Getters;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use dpp::ProtocolError;
use dpp::state_transition::batch_transition::batched_transition::document_delete_transition::DocumentDeleteTransitionV0;
use crate::drive::contract::DataContractFetchInfo;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use crate::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::v0::DocumentDeleteTransitionActionV0;
use crate::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;

impl DocumentDeleteTransitionActionV0 {
    /// try from
    pub fn try_from_document_delete_transition_with_contract_lookup(
        value: DocumentDeleteTransitionV0,
        owner_id: Identifier,
        user_fee_increase: UserFeeIncrease,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<
        (
            ConsensusValidationResult<BatchedTransitionAction>,
            FeeResult,
        ),
        Error,
    > {
        let DocumentDeleteTransitionV0 { base, .. } = value;

        Ok((
            BatchedTransitionAction::DocumentAction(DocumentTransitionAction::DeleteAction(
                DocumentDeleteTransitionActionV0 {
                    base: DocumentBaseTransitionAction::try_from_base_transition_with_contract_lookup(
                        base,
                        get_data_contract,
                        |document_type| document_type.document_deletion_token_cost(),
                    )?
                }
                
                    .into(),
            ))
                .into(),
            FeeResult::default(),
        ))
    }

    /// try from borrowed
    pub fn try_from_borrowed_document_delete_transition_with_contract_lookup(
        value: &DocumentDeleteTransitionV0,
        owner_id: Identifier,
        user_fee_increase: UserFeeIncrease,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<
        (
            ConsensusValidationResult<BatchedTransitionAction>,
            FeeResult,
        ),
        Error,
    > {
        let DocumentDeleteTransitionV0 { base, .. } = value;
        Ok((
            BatchedTransitionAction::DocumentAction(DocumentTransitionAction::DeleteAction(
                DocumentDeleteTransitionActionV0 {
                    base: DocumentBaseTransitionAction::try_from_borrowed_base_transition_with_contract_lookup(
                        base,
                        get_data_contract,
                        |document_type| document_type.document_deletion_token_cost(),
                    )?
                }

                    .into(),
            ))
                .into(),
            FeeResult::default(),
        ))
    }
}
