use dpp::block::block_info::BlockInfo;
use dpp::document::property_names::{CREATOR_ID, PRICE};
use dpp::document::{property_names, Document, DocumentV0Getters, DocumentV0Setters};
use dpp::platform_value::Identifier;
use std::sync::Arc;
use dpp::data_contract::document_type::accessors::DocumentTypeV1Getters;
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use dpp::ProtocolError;
use dpp::state_transition::batch_transition::batched_transition::document_purchase_transition::DocumentPurchaseTransitionV0;
use platform_version::version::PlatformVersion;
use crate::drive::contract::DataContractFetchInfo;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use crate::state_transition_action::batch::batched_transition::document_transition::document_purchase_transition_action::v0::DocumentPurchaseTransitionActionV0;
use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;

impl DocumentPurchaseTransitionActionV0 {
    /// try from borrowed
    pub fn try_from_borrowed_document_purchase_transition(
        document_purchase_transition: &DocumentPurchaseTransitionV0,
        owner_id: Identifier,
        original_document: Document,
        purchaser_id: Identifier,
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
        let DocumentPurchaseTransitionV0 { base, price, .. } = document_purchase_transition;
        let base_action_validation_result =
            DocumentBaseTransitionAction::try_from_borrowed_base_transition_with_contract_lookup(
                base,
                get_data_contract,
                |document_type| document_type.document_purchase_token_cost(),
                "purchase",
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
        let original_owner_id = original_document.owner_id();

        let mut modified_document = original_document;

        // If we don't have a creator id that means we never had sold it before
        if modified_document
            .properties()
            .get_optional_identifier(CREATOR_ID)?
            .is_none()
            && platform_version.protocol_version >= 10
        {
            modified_document
                .properties_mut()
                .insert(CREATOR_ID.to_string(), original_owner_id.into());
        }

        modified_document.bump_revision();

        // We must remove the price if there is one
        modified_document.properties_mut().remove(PRICE);

        modified_document.set_owner_id(purchaser_id);

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
            BatchedTransitionAction::DocumentAction(DocumentTransitionAction::PurchaseAction(
                DocumentPurchaseTransitionActionV0 {
                    base,
                    document: modified_document,
                    original_owner_id,
                    price: *price,
                }
                .into(),
            ))
            .into(),
            FeeResult::default(),
        ))
    }
}
