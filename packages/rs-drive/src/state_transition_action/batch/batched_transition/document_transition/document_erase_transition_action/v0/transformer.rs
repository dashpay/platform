use crate::drive::contract::DataContractFetchInfo;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use crate::state_transition_action::batch::batched_transition::document_transition::document_erase_transition_action::v0::DocumentEraseTransitionActionV0;
use crate::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;
use dpp::consensus::basic::document::InvalidDocumentTransitionActionError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::fee::fee_result::FeeResult;
use dpp::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::document_base_transition::v1::v1_methods::DocumentBaseTransitionV1Methods;
use dpp::platform_value::Identifier;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use dpp::state_transition::batch_transition::batched_transition::document_erase_transition::DocumentEraseTransitionV0;
use dpp::ProtocolError;
use std::sync::Arc;

impl DocumentEraseTransitionActionV0 {
    /// try from borrowed
    pub fn try_from_borrowed_document_erase_transition_with_contract_lookup(
        value: &DocumentEraseTransitionV0,
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
        let DocumentEraseTransitionV0 { base, .. } = value;

        // Erase carries no token cost, so an offer to pay one has nothing to
        // buy. Refused here rather than in structure validation because the
        // base action keeps only the cost the contract sets, not the payment
        // the submitter offered: by then there is nothing left to see.
        // Accepting it would let a continuation, which any identity may submit,
        // move somebody else's tokens.
        if base.token_payment_info_ref().is_some() {
            let bump_action =
                BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition(
                    base,
                    owner_id,
                    user_fee_increase,
                );
            return Ok((
                ConsensusValidationResult::new_with_data_and_errors(
                    BatchedTransitionAction::BumpIdentityDataContractNonce(bump_action),
                    vec![ConsensusError::BasicError(
                        BasicError::InvalidDocumentTransitionActionError(
                            InvalidDocumentTransitionActionError::new(format!(
                                "an erase of document {} must not carry token payment information",
                                base.id()
                            )),
                        ),
                    )],
                ),
                FeeResult::default(),
            ));
        }

        let base_action_validation_result =
            DocumentBaseTransitionAction::try_from_borrowed_base_transition_with_contract_lookup(
                base,
                get_data_contract,
                // Erase carries no token cost: the deletion it follows was
                // charged when the document was deleted.
                |_document_type| None,
                "erase",
            )?;

        let base = match base_action_validation_result.is_valid() {
            true => base_action_validation_result.into_data()?,
            false => {
                let bump_action =
                    BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition(
                        base,
                        owner_id,
                        user_fee_increase,
                    );
                let batched_action =
                    BatchedTransitionAction::BumpIdentityDataContractNonce(bump_action);

                return Ok((
                    ConsensusValidationResult::new_with_data_and_errors(
                        batched_action,
                        base_action_validation_result.errors,
                    ),
                    FeeResult::default(),
                ));
            }
        };

        Ok((
            BatchedTransitionAction::DocumentAction(DocumentTransitionAction::EraseAction(
                DocumentEraseTransitionActionV0 { base }.into(),
            ))
            .into(),
            FeeResult::default(),
        ))
    }
}
