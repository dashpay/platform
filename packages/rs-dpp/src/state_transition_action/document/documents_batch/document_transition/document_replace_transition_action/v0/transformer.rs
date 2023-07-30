use platform_value::Identifier;
use crate::data_contract::DataContract;
use crate::identity::TimestampMillis;
use crate::ProtocolError;
use crate::state_transition::documents_batch_transition::document_transition::document_replace_transition::DocumentReplaceTransitionV0;
use crate::state_transition_action::document::documents_batch::document_transition::document_replace_transition_action::v0::DocumentReplaceTransitionActionV0;
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionAction;

impl<'a> DocumentReplaceTransitionActionV0<'a> {
    pub(in crate::state_transition_action::document::documents_batch::document_transition) fn try_from_borrowed_document_replace_transition(
        document_replace_transition: &DocumentReplaceTransitionV0,
        originally_created_at: Option<TimestampMillis>,
        get_data_contract: impl FnMut(Identifier) -> Result<&'a DataContract, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        let DocumentReplaceTransitionV0 {
            base,
            revision,
            updated_at,
            data,
            ..
        } = document_replace_transition;
        Ok(DocumentReplaceTransitionActionV0 {
            base: DocumentBaseTransitionAction::from_borrowed_base_transition_with_contract_lookup(
                base,
                get_data_contract,
            )?,
            revision: *revision,
            created_at: originally_created_at,
            updated_at: *updated_at,
            //todo: remove clone
            data: data.clone().unwrap_or_default(),
        })
    }
}
