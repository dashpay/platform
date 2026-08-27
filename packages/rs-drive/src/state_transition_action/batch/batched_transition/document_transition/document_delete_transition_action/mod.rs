use derive_more::From;

use crate::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::v0::{DocumentDeleteTransitionActionAccessorsV0, DocumentDeleteTransitionActionV0};
use crate::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::v1::{DocumentDeleteTransitionActionAccessorsV1, DocumentDeleteTransitionActionV1};

/// transformer
pub mod transformer;
/// v0
pub mod v0;
/// v1 (indexOnly delete-by-values)
pub mod v1;

use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use dpp::platform_value::Value;
use std::collections::BTreeMap;

/// document delete transition action
#[derive(Debug, Clone, From)]
pub enum DocumentDeleteTransitionAction {
    /// v0
    V0(DocumentDeleteTransitionActionV0),
    /// v1 — indexOnly delete-by-values: carries the document's property
    /// values, since there is no primary-storage row to fetch them from
    V1(DocumentDeleteTransitionActionV1),
}

impl DocumentDeleteTransitionActionAccessorsV0 for DocumentDeleteTransitionAction {
    fn base(&self) -> &DocumentBaseTransitionAction {
        match self {
            DocumentDeleteTransitionAction::V0(v0) => &v0.base,
            DocumentDeleteTransitionAction::V1(v1) => &v1.base,
        }
    }

    fn base_owned(self) -> DocumentBaseTransitionAction {
        match self {
            DocumentDeleteTransitionAction::V0(v0) => v0.base,
            DocumentDeleteTransitionAction::V1(v1) => v1.base,
        }
    }
}

impl DocumentDeleteTransitionActionAccessorsV1 for DocumentDeleteTransitionAction {
    fn data(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            DocumentDeleteTransitionAction::V0(_) => None,
            DocumentDeleteTransitionAction::V1(v1) => Some(&v1.data),
        }
    }
}

impl DocumentDeleteTransitionAction {
    /// Consume the action into its base and (for V1) the indexOnly values.
    pub fn base_and_data_owned(
        self,
    ) -> (
        DocumentBaseTransitionAction,
        Option<BTreeMap<String, Value>>,
    ) {
        match self {
            DocumentDeleteTransitionAction::V0(v0) => (v0.base, None),
            DocumentDeleteTransitionAction::V1(v1) => (v1.base, Some(v1.data)),
        }
    }
}
