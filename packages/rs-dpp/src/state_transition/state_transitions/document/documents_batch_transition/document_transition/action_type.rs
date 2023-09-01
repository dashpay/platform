use crate::state_transition::documents_batch_transition::document_transition::DocumentTransition;
use crate::ProtocolError;

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

impl TryFrom<&str> for DocumentTransitionActionType {
    type Error = ProtocolError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "create" => Ok(DocumentTransitionActionType::Create),
            "replace" => Ok(DocumentTransitionActionType::Replace),
            "delete" => Ok(DocumentTransitionActionType::Delete),
            action_type => Err(ProtocolError::Generic(format!(
                "unknown action type {action_type}"
            ))),
        }
    }
}
