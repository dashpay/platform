use dpp::block::block_info::BlockInfo;
use dpp::document::property_names;
use dpp::platform_value::Identifier;
use std::sync::Arc;

use dpp::identity::TimestampMillis;
use dpp::prelude::{BlockHeight, CoreBlockHeight};
use dpp::ProtocolError;
use dpp::state_transition::documents_batch_transition::document_transition::document_replace_transition::DocumentReplaceTransitionV0;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::document::documents_batch::document_transition::document_replace_transition_action::v0::DocumentReplaceTransitionActionV0;
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionAccessorsV0};

impl DocumentReplaceTransitionActionV0 {
    /// try from borrowed
    pub fn try_from_borrowed_document_replace_transition(
        document_replace_transition: &DocumentReplaceTransitionV0,
        originally_created_at: Option<TimestampMillis>,
        originally_created_at_block_height: Option<BlockHeight>,
        originally_created_at_core_block_height: Option<CoreBlockHeight>,
        originally_transferred_at: Option<TimestampMillis>,
        originally_transferred_at_block_height: Option<BlockHeight>,
        originally_transferred_at_core_block_height: Option<CoreBlockHeight>,
        block_info: &BlockInfo,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        let DocumentReplaceTransitionV0 {
            base,
            revision,
            data,
            ..
        } = document_replace_transition;
        let base =
            DocumentBaseTransitionAction::from_borrowed_base_transition_with_contract_lookup(
                base,
                get_data_contract,
            )?;
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

        Ok(DocumentReplaceTransitionActionV0 {
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
        })
    }
}
