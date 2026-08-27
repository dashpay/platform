use derive_more::From;

use crate::state_transition_action::batch::batched_transition::document_transition::document_index_only_delete_transition_action::v0::{DocumentIndexOnlyDeleteTransitionActionAccessorsV0, DocumentIndexOnlyDeleteTransitionActionV0};

/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use dpp::platform_value::Value;
use std::collections::BTreeMap;

/// The indexOnly delete action: the base plus the document's full
/// property-value tuple (including `$createdAt` under its system key when
/// the type requires it). There is no primary-storage row to fetch — the
/// values are what every index entry is recomputed from.
#[derive(Debug, Clone, From)]
pub enum DocumentIndexOnlyDeleteTransitionAction {
    /// v0
    V0(DocumentIndexOnlyDeleteTransitionActionV0),
}

impl DocumentIndexOnlyDeleteTransitionActionAccessorsV0
    for DocumentIndexOnlyDeleteTransitionAction
{
    fn base(&self) -> &DocumentBaseTransitionAction {
        match self {
            DocumentIndexOnlyDeleteTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> DocumentBaseTransitionAction {
        match self {
            DocumentIndexOnlyDeleteTransitionAction::V0(v0) => v0.base,
        }
    }

    fn data(&self) -> &BTreeMap<String, Value> {
        match self {
            DocumentIndexOnlyDeleteTransitionAction::V0(v0) => &v0.data,
        }
    }
}

impl DocumentIndexOnlyDeleteTransitionAction {
    /// Consume the action into its base and the indexOnly values.
    pub fn base_and_data_owned(self) -> (DocumentBaseTransitionAction, BTreeMap<String, Value>) {
        match self {
            DocumentIndexOnlyDeleteTransitionAction::V0(v0) => (v0.base, v0.data),
        }
    }
}
