use dpp::block::block_info::BlockInfo;
use dpp::document::property_names;
use dpp::platform_value::Identifier;
use std::sync::Arc;
use dpp::data_contract::document_type::accessors::DocumentTypeV1Getters;
use dpp::fee::fee_result::FeeResult;
use dpp::identity::TimestampMillis;
use dpp::prelude::{BlockHeight, ConsensusValidationResult, CoreBlockHeight, UserFeeIncrease};
use dpp::ProtocolError;
use dpp::state_transition::batch_transition::batched_transition::document_replace_transition::DocumentReplaceTransitionV0;
use crate::drive::contract::DataContractFetchInfo;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use crate::state_transition_action::batch::batched_transition::document_transition::document_replace_transition_action::v0::DocumentReplaceTransitionActionV0;
use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;

impl DocumentReplaceTransitionActionV0 {
    /// try from borrowed
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_borrowed_document_replace_transition(
        document_replace_transition: &DocumentReplaceTransitionV0,
        owner_id: Identifier,
        originally_created_at: Option<TimestampMillis>,
        originally_created_at_block_height: Option<BlockHeight>,
        originally_created_at_core_block_height: Option<CoreBlockHeight>,
        originally_transferred_at: Option<TimestampMillis>,
        originally_transferred_at_block_height: Option<BlockHeight>,
        originally_transferred_at_core_block_height: Option<CoreBlockHeight>,
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
        let DocumentReplaceTransitionV0 {
            base,
            revision,
            data,
            ..
        } = document_replace_transition;
        let base_action_validation_result =
            DocumentBaseTransitionAction::try_from_borrowed_base_transition_with_contract_lookup(
                base,
                get_data_contract,
                |document_type| document_type.document_replacement_token_cost(),
                "replace",
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
        let updated_at = if base.document_type_field_is_required(property_names::UPDATED_AT)? {
            Some(block_info.time_ms)
        } else {
            None
        };

        let updated_at_block_height =
            if base.document_type_field_is_required(property_names::UPDATED_AT_BLOCK_HEIGHT)? {
                Some(block_info.height)
            } else {
                None
            };

        let updated_at_core_block_height = if base
            .document_type_field_is_required(property_names::UPDATED_AT_CORE_BLOCK_HEIGHT)?
        {
            Some(block_info.core_height)
        } else {
            None
        };

        Ok((
            BatchedTransitionAction::DocumentAction(DocumentTransitionAction::ReplaceAction(
                DocumentReplaceTransitionActionV0 {
                    base,
                    revision: *revision,
                    created_at: originally_created_at,
                    updated_at,
                    transferred_at: originally_transferred_at,
                    created_at_block_height: originally_created_at_block_height,
                    updated_at_block_height,
                    transferred_at_block_height: originally_transferred_at_block_height,
                    created_at_core_block_height: originally_created_at_core_block_height,
                    updated_at_core_block_height,
                    transferred_at_core_block_height: originally_transferred_at_core_block_height,
                    data: data.clone(),
                }
                .into(),
            ))
            .into(),
            FeeResult::default(),
        ))
    }
}
