use dpp::block::block_info::BlockInfo;
use dpp::document::{property_names, Document, DocumentV0Getters};
use dpp::platform_value::{Identifier, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use dpp::data_contract::document_type::accessors::DocumentTypeV1Getters;
use dpp::data_contract::document_type::methods::DocumentTypeBasicMethods;
use platform_version::version::PlatformVersion;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
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
        original_document: &Document,
        block_info: &BlockInfo,
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
                |action_fees| action_fees.document_replacement_action_fee(),
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
        // Added in place at protocol version 14, inert before it: `fill_generated_properties`
        // is `None` there and leaves the data as sent. From 14 on, every `generatedFrom`
        // property the transition leaves out is generated from its params here, before the
        // changed fields below and every later check read the data. `document_type()`
        // cannot fail here at any version: building the base action above already resolved
        // the document type (its token cost is read from it), and the
        // `document_type_field_is_required` calls below resolve it the same way.
        let mut data = data.clone();
        base.document_type()?
            .fill_generated_properties(&mut data, platform_version)?;

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

        // There is a case where we updated a just deleted document
        // In this case we don't care about the created at
        let original_document_created_at = original_document.created_at();

        let original_document_created_at_block_height = original_document.created_at_block_height();

        let original_document_created_at_core_block_height =
            original_document.created_at_core_block_height();

        let original_document_transferred_at = original_document.transferred_at();

        let original_document_transferred_at_block_height =
            original_document.transferred_at_block_height();

        let original_document_transferred_at_core_block_height =
            original_document.transferred_at_core_block_height();

        let original_creator_id = original_document.creator_id();

        // The last moderator's stamp stays as it is, each half as stored, unless the
        // transformer stamps the replace as a moderator's
        let original_moderated_at = original_document.moderated_at();
        let original_moderated_by = original_document.moderated_by();

        // The identifier each removed field held. Kept because a removed
        // `deletableDocument` reference is only allowed on an `immutable`
        // property once its target is gone, and after this point nothing
        // else remembers what the target was.
        let removed_identifier_fields: BTreeMap<String, Identifier> = original_document
            .properties()
            .iter()
            .filter(|(key, _)| !data.contains_key(*key))
            .filter_map(|(key, value)| {
                value
                    .to_identifier()
                    .ok()
                    .map(|identifier| (key.clone(), identifier))
            })
            .collect();

        // Determine which fields have changed between the original document and the new data
        let changed_fields: BTreeSet<String> = data
            .iter()
            .filter_map(|(key, new_value)| {
                let original_value = original_document.properties().get(key);
                match original_value {
                    Some(old_value) => {
                        if !old_value.equal_underlying_data(new_value) {
                            Some(key.clone())
                        } else {
                            None
                        }
                    }
                    None => Some(key.clone()), // New field that wasn't in original
                }
            })
            .chain(
                // Check for fields that were in the original but removed in the new data
                original_document.properties().keys().filter_map(|key| {
                    if !data.contains_key(key) {
                        Some(key.clone())
                    } else {
                        None
                    }
                }),
            )
            .collect();

        // What the stored document held for each changed property, so the
        // reference validation can tell a list's new elements from the ones
        // it already held
        let stored_changed_values: BTreeMap<String, Value> = changed_fields
            .iter()
            .filter_map(|key| {
                original_document
                    .properties()
                    .get(key)
                    .map(|value| (key.clone(), value.clone()))
            })
            .collect();

        Ok((
            BatchedTransitionAction::DocumentAction(DocumentTransitionAction::ReplaceAction(
                DocumentReplaceTransitionActionV0 {
                    base,
                    revision: *revision,
                    created_at: original_document_created_at,
                    updated_at,
                    transferred_at: original_document_transferred_at,
                    created_at_block_height: original_document_created_at_block_height,
                    updated_at_block_height,
                    transferred_at_block_height: original_document_transferred_at_block_height,
                    created_at_core_block_height: original_document_created_at_core_block_height,
                    updated_at_core_block_height,
                    transferred_at_core_block_height:
                        original_document_transferred_at_core_block_height,
                    data,
                    changed_data_fields: changed_fields,
                    removed_identifier_fields,
                    stored_changed_values,
                    creator_id: original_creator_id,
                    moderated_at: original_moderated_at,
                    moderated_by: original_moderated_by,
                    property_constraint_aggregates: Default::default(),
                }
                .into(),
            ))
            .into(),
            FeeResult::default(),
        ))
    }
}
