use crate::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;
use dpp::state_transition::batch_transition::batched_transition::document_transition_action_type::{
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
