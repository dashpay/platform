use crate::state_transition::state_transitions::document::batch_transition::batched_transition::document_transition::DocumentTransition;
use crate::ProtocolError;

// @append-only
#[derive(Eq, PartialEq, Debug, Copy, Clone, Hash)]
pub enum DocumentTransitionActionType {
    Create, //the entropy used
    Replace,
    Delete,
    Transfer,
    Purchase,
    UpdatePrice,
    IgnoreWhileBumpingRevision,
    IndexOnlyDelete,
}

pub trait DocumentTransitionActionTypeGetter {
    fn action_type(&self) -> DocumentTransitionActionType;
}

impl DocumentTransitionActionTypeGetter for DocumentTransition {
    fn action_type(&self) -> DocumentTransitionActionType {
        match self {
            DocumentTransition::Create(_) => DocumentTransitionActionType::Create,
            DocumentTransition::Delete(_) => DocumentTransitionActionType::Delete,
            DocumentTransition::Replace(_) => DocumentTransitionActionType::Replace,
            DocumentTransition::Transfer(_) => DocumentTransitionActionType::Transfer,
            DocumentTransition::UpdatePrice(_) => DocumentTransitionActionType::UpdatePrice,
            DocumentTransition::Purchase(_) => DocumentTransitionActionType::Purchase,
            DocumentTransition::IndexOnlyDelete(_) => DocumentTransitionActionType::IndexOnlyDelete,
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
            "transfer" => Ok(DocumentTransitionActionType::Transfer),
            "updatePrice" | "update_price" => Ok(DocumentTransitionActionType::UpdatePrice),
            "purchase" => Ok(DocumentTransitionActionType::Purchase),
            "indexOnlyDelete" | "index_only_delete" => {
                Ok(DocumentTransitionActionType::IndexOnlyDelete)
            }
            action_type => Err(ProtocolError::Generic(format!(
                "unknown action type {action_type}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The happy paths and basic error case are already covered in
    // batch_transition/tests.rs. The tests below add coverage for
    // edge cases NOT exercised there.

    #[test]
    fn try_from_str_returns_protocol_error_with_unknown_action_substring() {
        // Verify the specific error message structure — exercises the
        // format!() inside the catch-all arm.
        let err = DocumentTransitionActionType::try_from("nonexistent").unwrap_err();
        match err {
            ProtocolError::Generic(msg) => {
                assert!(msg.contains("unknown action type"));
                assert!(msg.contains("nonexistent"));
            }
            other => panic!("expected ProtocolError::Generic, got {:?}", other),
        }
    }

    #[test]
    fn try_from_str_is_case_sensitive() {
        // Only lowercase "create" is valid; capitalized variants must error.
        assert!(DocumentTransitionActionType::try_from("Create").is_err());
        assert!(DocumentTransitionActionType::try_from("CREATE").is_err());
        assert!(DocumentTransitionActionType::try_from("DELETE").is_err());
    }

    #[test]
    fn try_from_str_errors_for_empty_string() {
        assert!(DocumentTransitionActionType::try_from("").is_err());
    }

    #[test]
    fn try_from_str_does_not_trim_whitespace() {
        // Whitespace variants are not trimmed
        assert!(DocumentTransitionActionType::try_from(" create").is_err());
        assert!(DocumentTransitionActionType::try_from("create ").is_err());
        assert!(DocumentTransitionActionType::try_from("\tcreate").is_err());
    }

    #[test]
    fn try_from_str_rejects_internal_bump_revision_variant() {
        // `IgnoreWhileBumpingRevision` is an internal variant with no string form.
        assert!(DocumentTransitionActionType::try_from("ignoreWhileBumpingRevision").is_err());
        assert!(DocumentTransitionActionType::try_from("ignore_while_bumping_revision").is_err());
    }
}
