use crate::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;
use dpp::identifier::Identifier;
use dpp::prelude::UserFeeMultiplier;

/// action v0
#[derive(Default, Debug, Clone)]
pub struct DocumentsBatchTransitionActionV0 {
    /// The owner making the transitions
    pub owner_id: Identifier,
    /// The inner transitions
    pub transitions: Vec<DocumentTransitionAction>,
    /// fee multiplier
    pub fee_multiplier: UserFeeMultiplier,
}
