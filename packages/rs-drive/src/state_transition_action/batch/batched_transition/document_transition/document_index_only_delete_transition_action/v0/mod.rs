/// transformer
pub mod transformer;

use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use dpp::platform_value::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
/// document indexOnly delete transition action v0
pub struct DocumentIndexOnlyDeleteTransitionActionV0 {
    /// base
    pub base: DocumentBaseTransitionAction,
    /// the property values of the indexOnly document being deleted
    pub data: BTreeMap<String, Value>,
}

/// document indexOnly delete transition action accessors v0
pub trait DocumentIndexOnlyDeleteTransitionActionAccessorsV0 {
    /// base
    fn base(&self) -> &DocumentBaseTransitionAction;
    /// base owned
    fn base_owned(self) -> DocumentBaseTransitionAction;
    /// the property values of the indexOnly document being deleted
    /// (`$createdAt` rides in the map under its system key when the
    /// document type requires it)
    fn data(&self) -> &BTreeMap<String, Value>;
}
