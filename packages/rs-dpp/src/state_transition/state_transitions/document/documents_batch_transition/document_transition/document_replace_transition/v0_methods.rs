use std::collections::BTreeMap;
use platform_value::Value;
use crate::identity::TimestampMillis;
use crate::prelude::{IdentityNonce, Revision};
use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
use crate::state_transition::documents_batch_transition::document_transition::document_replace_transition::v0::v0_methods::DocumentReplaceTransitionV0Methods;
use crate::state_transition::documents_batch_transition::document_transition::DocumentReplaceTransition;

impl DocumentReplaceTransitionV0Methods for DocumentReplaceTransition {
    fn base(&self) -> &DocumentBaseTransition {
        match self {
            DocumentReplaceTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut DocumentBaseTransition {
        match self {
            DocumentReplaceTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: DocumentBaseTransition) {
        match self {
            DocumentReplaceTransition::V0(v0) => v0.base = base,
        }
    }

    fn revision(&self) -> Revision {
        match self {
            DocumentReplaceTransition::V0(v0) => v0.revision,
        }
    }

    fn set_revision(&mut self, revision: Revision) {
        match self {
            DocumentReplaceTransition::V0(v0) => v0.revision = revision,
        }
    }

    fn updated_at(&self) -> Option<TimestampMillis> {
        match self {
            DocumentReplaceTransition::V0(v0) => v0.updated_at,
        }
    }

    fn set_updated_at(&mut self, updated_at: Option<TimestampMillis>) {
        match self {
            DocumentReplaceTransition::V0(v0) => v0.updated_at = updated_at,
        }
    }

    fn data(&self) -> &BTreeMap<String, Value> {
        match self {
            DocumentReplaceTransition::V0(v0) => &v0.data,
        }
    }

    fn data_mut(&mut self) -> &mut BTreeMap<String, Value> {
        match self {
            DocumentReplaceTransition::V0(v0) => &mut v0.data,
        }
    }

    fn set_data(&mut self, data: BTreeMap<String, Value>) {
        match self {
            DocumentReplaceTransition::V0(v0) => v0.data = data,
        }
    }
}
