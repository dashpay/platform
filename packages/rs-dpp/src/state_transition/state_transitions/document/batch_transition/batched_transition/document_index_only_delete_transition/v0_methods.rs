use platform_value::Value;
use std::collections::BTreeMap;

use crate::state_transition::batch_transition::batched_transition::document_index_only_delete_transition::v0::v0_methods::DocumentIndexOnlyDeleteTransitionV0Methods;
use crate::state_transition::batch_transition::batched_transition::DocumentIndexOnlyDeleteTransition;
use crate::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;

impl DocumentBaseTransitionAccessors for DocumentIndexOnlyDeleteTransition {
    fn base(&self) -> &DocumentBaseTransition {
        match self {
            DocumentIndexOnlyDeleteTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut DocumentBaseTransition {
        match self {
            DocumentIndexOnlyDeleteTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: DocumentBaseTransition) {
        match self {
            DocumentIndexOnlyDeleteTransition::V0(v0) => v0.base = base,
        }
    }
}

impl DocumentIndexOnlyDeleteTransitionV0Methods for DocumentIndexOnlyDeleteTransition {
    fn data(&self) -> &BTreeMap<String, Value> {
        match self {
            DocumentIndexOnlyDeleteTransition::V0(v0) => &v0.data,
        }
    }

    fn data_mut(&mut self) -> &mut BTreeMap<String, Value> {
        match self {
            DocumentIndexOnlyDeleteTransition::V0(v0) => &mut v0.data,
        }
    }

    fn set_data(&mut self, data: BTreeMap<String, Value>) {
        match self {
            DocumentIndexOnlyDeleteTransition::V0(v0) => v0.data = data,
        }
    }
}
