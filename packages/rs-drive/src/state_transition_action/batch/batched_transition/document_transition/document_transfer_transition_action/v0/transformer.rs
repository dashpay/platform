use dpp::block::block_info::BlockInfo;
use dpp::document::property_names::PRICE;
use dpp::document::{property_names, Document, DocumentV0Getters, DocumentV0Setters};
use dpp::platform_value::Identifier;
use std::sync::Arc;
use dpp::data_contract::document_type::accessors::DocumentTypeV1Getters;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use dpp::ProtocolError;
use dpp::state_transition::batch_transition::batched_transition::document_transfer_transition::DocumentTransferTransitionV0;
use crate::drive::contract::DataContractFetchInfo;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use crate::state_transition_action::batch::batched_transition::document_transition::document_transfer_transition_action::v0::DocumentTransferTransitionActionV0;
use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;

impl DocumentTransferTransitionActionV0 {
    /// try from borrowed
    pub fn try_from_borrowed_document_transfer_transition(
        document_transfer_transition: &DocumentTransferTransitionV0,
        owner_id: Identifier,
        original_document: Document,
        block_info: &BlockInfo,
        user_fee_increase: UserFeeIncrease,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<
        (
            ConsensusValidationResult<BatchedTransitionAction>,
            FeeResult,
        ),
        Error,
    > {
        let DocumentTransferTransitionV0 {
            base,
            recipient_owner_id,
            ..
        } = document_transfer_transition;
        let base_action_validation_result =
            DocumentBaseTransitionAction::try_from_borrowed_base_transition_with_contract_lookup(
                base,
                get_data_contract,
                |document_type| document_type.document_transfer_token_cost(),
                "transfer",
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

        let mut modified_document = original_document;

        modified_document.set_owner_id(*recipient_owner_id);

        // We must remove the price
        modified_document.properties_mut().remove(PRICE);

        modified_document.bump_revision();

        if base.document_type_field_is_required(property_names::TRANSFERRED_AT)? {
            modified_document.set_transferred_at(Some(block_info.time_ms));
        }

        if base.document_type_field_is_required(property_names::TRANSFERRED_AT_BLOCK_HEIGHT)? {
            modified_document.set_transferred_at_block_height(Some(block_info.height));
        }

        if base.document_type_field_is_required(property_names::TRANSFERRED_AT_CORE_BLOCK_HEIGHT)? {
            modified_document.set_transferred_at_core_block_height(Some(block_info.core_height));
        }

        Ok((
            BatchedTransitionAction::DocumentAction(DocumentTransitionAction::TransferAction(
                DocumentTransferTransitionActionV0 {
                    base,
                    document: modified_document,
                }
                .into(),
            ))
            .into(),
            FeeResult::default(),
        ))
    }
}
