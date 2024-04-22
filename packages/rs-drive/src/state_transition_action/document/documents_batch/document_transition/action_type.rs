use crate::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;
use dpp::state_transition::documents_batch_transition::document_transition::action_type::{
    DocumentTransitionActionType, TransitionActionTypeGetter,
};

impl TransitionActionTypeGetter for DocumentTransitionAction {
    fn action_type(&self) -> DocumentTransitionActionType {
        match self {
            DocumentTransitionAction::CreateAction(_) => DocumentTransitionActionType::Create,
            DocumentTransitionAction::DeleteAction(_) => DocumentTransitionActionType::Delete,
            DocumentTransitionAction::ReplaceAction(_) => DocumentTransitionActionType::Replace,
            DocumentTransitionAction::TransferAction(_) => DocumentTransitionActionType::Transfer,
            DocumentTransitionAction::PurchaseAction(_) => DocumentTransitionActionType::Purchase,
            DocumentTransitionAction::UpdatePriceAction(_) => {
                DocumentTransitionActionType::UpdatePrice
            }
            DocumentTransitionAction::BumpIdentityDataContractNonce(_) => {
                DocumentTransitionActionType::IgnoreWhileBumpingRevision
            }
        }
    }
}
