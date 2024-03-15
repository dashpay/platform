use dpp::platform_value::Identifier;
use std::sync::Arc;

use dpp::identity::TimestampMillis;
use dpp::ProtocolError;
use dpp::state_transition::documents_batch_transition::document_transition::DocumentReplaceTransition;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::document::documents_batch::document_transition::document_replace_transition_action::{DocumentReplaceTransitionAction, DocumentReplaceTransitionActionV0};

impl DocumentReplaceTransitionAction {
    /// try from borrowed
    pub fn try_from_borrowed_document_replace_transition(
        document_replace_transition: &DocumentReplaceTransition,
        originally_created_at: Option<TimestampMillis>,
        block_time_ms: TimestampMillis,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        match document_replace_transition {
            DocumentReplaceTransition::V0(v0) => Ok(
                DocumentReplaceTransitionActionV0::try_from_borrowed_document_replace_transition(
                    v0,
                    originally_created_at,
                    block_time_ms,
                    get_data_contract,
                )?
                .into(),
            ),
        }
    }
}
