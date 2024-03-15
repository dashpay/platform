use dpp::document::property_names;
use dpp::platform_value::Identifier;
use std::sync::Arc;

use dpp::identity::TimestampMillis;
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
        block_time_ms: TimestampMillis,
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
            Some(block_time_ms)
        } else {
            None
        };

        Ok(DocumentReplaceTransitionActionV0 {
            base,
            revision: *revision,
            created_at: originally_created_at,
            updated_at,
            data: data.clone(),
        })
    }
}
