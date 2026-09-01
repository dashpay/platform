use platform_value::Value;
use std::collections::BTreeMap;

use crate::state_transition::batch_transition::batched_transition::document_index_only_delete_transition::DocumentIndexOnlyDeleteTransitionV0;
use crate::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;

pub trait DocumentIndexOnlyDeleteTransitionV0Methods: DocumentBaseTransitionAccessors {
    /// Returns a reference to the property values of the indexOnly
    /// document being deleted (`$createdAt` rides in the map under its
    /// system key when the document type requires it).
    fn data(&self) -> &BTreeMap<String, Value>;

    /// Returns a mutable reference to the property values.
    fn data_mut(&mut self) -> &mut BTreeMap<String, Value>;

    /// Sets the property values.
    fn set_data(&mut self, data: BTreeMap<String, Value>);
}

impl DocumentBaseTransitionAccessors for DocumentIndexOnlyDeleteTransitionV0 {
    fn base(&self) -> &DocumentBaseTransition {
        &self.base
    }

    fn base_mut(&mut self) -> &mut DocumentBaseTransition {
        &mut self.base
    }

    fn set_base(&mut self, base: DocumentBaseTransition) {
        self.base = base
    }
}

impl DocumentIndexOnlyDeleteTransitionV0Methods for DocumentIndexOnlyDeleteTransitionV0 {
    fn data(&self) -> &BTreeMap<String, Value> {
        &self.data
    }

    fn data_mut(&mut self) -> &mut BTreeMap<String, Value> {
        &mut self.data
    }

    fn set_data(&mut self, data: BTreeMap<String, Value>) {
        self.data = data;
    }
}
