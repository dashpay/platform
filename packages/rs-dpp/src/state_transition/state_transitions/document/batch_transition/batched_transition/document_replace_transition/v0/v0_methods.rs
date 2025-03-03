use platform_value::Value;

use std::collections::BTreeMap;

use crate::prelude::Revision;
use crate::state_transition::state_transitions::document::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use crate::state_transition::state_transitions::document::batch_transition::document_base_transition::DocumentBaseTransition;
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::document_replace_transition::DocumentReplaceTransitionV0;

pub trait DocumentReplaceTransitionV0Methods: DocumentBaseTransitionAccessors {
    /// Returns a reference to the `revision` field of the `DocumentReplaceTransitionV0`.
    fn revision(&self) -> Revision;

    /// Sets the value of the `revision` field in the `DocumentReplaceTransitionV0`.
    fn set_revision(&mut self, revision: Revision);

    /// Returns a reference to the `data` field of the `DocumentReplaceTransitionV0`.
    fn data(&self) -> &BTreeMap<String, Value>;

    /// Returns a mutable reference to the `data` field of the `DocumentReplaceTransitionV0`.
    fn data_mut(&mut self) -> &mut BTreeMap<String, Value>;

    /// Sets the value of the `data` field in the `DocumentReplaceTransitionV0`.
    fn set_data(&mut self, data: BTreeMap<String, Value>);
}

impl DocumentBaseTransitionAccessors for DocumentReplaceTransitionV0 {
    fn base(&self) -> &DocumentBaseTransition {
        &self.base
    }

    fn base_mut(&mut self) -> &mut DocumentBaseTransition {
        &mut self.base
    }

    fn set_base(&mut self, base: DocumentBaseTransition) {
        self.base = base;
    }
}

impl DocumentReplaceTransitionV0Methods for DocumentReplaceTransitionV0 {
    fn revision(&self) -> Revision {
        self.revision
    }

    fn set_revision(&mut self, revision: Revision) {
        self.revision = revision;
    }

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
