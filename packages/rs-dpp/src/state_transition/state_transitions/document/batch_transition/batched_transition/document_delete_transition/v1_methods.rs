use crate::state_transition::batch_transition::batched_transition::DocumentDeleteTransition;
use platform_value::Value;
use std::collections::BTreeMap;

/// V1 (indexOnly) accessors on the delete transition enum. V0 arms return
/// `None` — a stored-document delete carries no values, same convention as
/// the base transition's `V1Methods`.
pub trait DocumentDeleteTransitionV1Methods {
    /// The property values of the indexOnly document being deleted, or
    /// `None` on a V0 (stored-document) delete.
    fn data(&self) -> Option<&BTreeMap<String, Value>>;
}

impl DocumentDeleteTransitionV1Methods for DocumentDeleteTransition {
    fn data(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            DocumentDeleteTransition::V0(_) => None,
            DocumentDeleteTransition::V1(v1) => Some(&v1.data),
        }
    }
}
