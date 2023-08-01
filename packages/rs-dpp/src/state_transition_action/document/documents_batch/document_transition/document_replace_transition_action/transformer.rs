use platform_value::Identifier;
use crate::data_contract::DataContract;
use crate::identity::TimestampMillis;
use crate::ProtocolError;
use crate::state_transition::documents_batch_transition::document_transition::DocumentReplaceTransition;
use crate::state_transition_action::document::documents_batch::document_transition::document_replace_transition_action::{DocumentReplaceTransitionAction, DocumentReplaceTransitionActionV0};

impl<'a> DocumentReplaceTransitionAction<'a> {
    pub fn try_from_borrowed_document_replace_transition(
        document_replace_transition: &DocumentReplaceTransition,
        originally_created_at: Option<TimestampMillis>,
        get_data_contract: impl Fn(Identifier) -> Result<&'a DataContract, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        match document_replace_transition {
            DocumentReplaceTransition::V0(v0) => Ok(
                DocumentReplaceTransitionActionV0::try_from_borrowed_document_replace_transition(
                    v0,
                    originally_created_at,
                    get_data_contract,
                )?
                .into(),
            ),
        }
    }
}
