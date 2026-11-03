use dpp::platform_value::Identifier;
use std::sync::Arc;
use dpp::data_contract::document_type::accessors::DocumentTypeV1Getters;
use dpp::data_contract::document_type::methods::DocumentTypeBasicMethods;
use platform_version::version::PlatformVersion;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use dpp::ProtocolError;
use dpp::state_transition::batch_transition::batched_transition::document_index_only_delete_transition::DocumentIndexOnlyDeleteTransitionV0;
use crate::drive::contract::DataContractFetchInfo;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::document_transition::document_index_only_delete_transition_action::v0::DocumentIndexOnlyDeleteTransitionActionV0;
use crate::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;

impl DocumentIndexOnlyDeleteTransitionActionV0 {
    /// try from borrowed — the indexOnly mirror of the delete action's V0
    /// transformer, additionally carrying the transition's values.
    pub fn try_from_borrowed_document_index_only_delete_transition_with_contract_lookup(
        value: &DocumentIndexOnlyDeleteTransitionV0,
        owner_id: Identifier,
        user_fee_increase: UserFeeIncrease,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<
        (
            ConsensusValidationResult<BatchedTransitionAction>,
            FeeResult,
        ),
        Error,
    > {
        let DocumentIndexOnlyDeleteTransitionV0 { base, data } = value;

        let base_action_validation_result =
            DocumentBaseTransitionAction::try_from_borrowed_base_transition_with_contract_lookup(
                base,
                get_data_contract,
                |document_type| document_type.document_deletion_token_cost(),
                |action_fees| action_fees.document_deletion_action_fee(),
                "indexOnlyDelete",
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

        // Added in place at protocol version 14, inert before it: `fill_generated_properties`
        // is `None` there and leaves the values as sent. From 14 on, the values name an
        // entry the way its create stored it: a `generatedFrom` property left out is
        // generated from its params, as the create generated it. `document_type()` cannot
        // fail here: building the base action above already resolved the document type
        // (its deletion token cost is read from it).
        let mut data = data.clone();
        base.document_type()?
            .fill_generated_properties(&mut data, platform_version)?;

        Ok((
            BatchedTransitionAction::DocumentAction(
                DocumentTransitionAction::IndexOnlyDeleteAction(
                    DocumentIndexOnlyDeleteTransitionActionV0 { base, data }.into(),
                ),
            )
            .into(),
            FeeResult::default(),
        ))
    }
}
