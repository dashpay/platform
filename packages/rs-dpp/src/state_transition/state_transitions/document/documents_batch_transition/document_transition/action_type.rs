use crate::state_transition::documents_batch_transition::document_transition::DocumentTransition;

// @append-only
#[derive(Eq, PartialEq, Debug, Copy, Clone, Hash)]
pub enum DocumentTransitionActionType {
    Create, //the entropy used
    Replace,
    Delete,
}

pub trait TransitionActionTypeGetter {
    fn action_type(&self) -> DocumentTransitionActionType;
}

impl TransitionActionTypeGetter for DocumentTransition {
    fn action_type(&self) -> DocumentTransitionActionType {
        match self {
            DocumentTransition::Create(_) => DocumentTransitionActionType::Create,
            DocumentTransition::Delete(_) => DocumentTransitionActionType::Delete,
            DocumentTransition::Replace(_) => DocumentTransitionActionType::Replace,
        }
    }
}
