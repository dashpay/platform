/// transformer
pub mod transformer;

use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use dpp::platform_value::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
/// The indexOnly delete action: the base plus the document's full
/// property-value tuple (including `$createdAt` under its system key when
/// the type indexes it). There is no primary-storage row to fetch — the
/// values are what every index entry is recomputed from.
pub struct DocumentDeleteTransitionActionV1 {
    /// base
    pub base: DocumentBaseTransitionAction,
    /// the property values of the indexOnly document being deleted
    pub data: BTreeMap<String, Value>,
}

/// document delete transition action v1 accessors
pub trait DocumentDeleteTransitionActionAccessorsV1 {
    /// The property values of the indexOnly document being deleted, or
    /// `None` on a V0 (stored-document) delete action.
    fn data(&self) -> Option<&BTreeMap<String, Value>>;
}
