use std::collections::BTreeMap;
use platform_value::{Value};
use crate::identity::TimestampMillis;
use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
use crate::state_transition::documents_batch_transition::document_create_transition::DocumentCreateTransition;
use crate::state_transition::documents_batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;

impl DocumentCreateTransitionV0Methods for DocumentCreateTransition {
    fn base(&self) -> &DocumentBaseTransition {
        match self {
            DocumentCreateTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut DocumentBaseTransition {
        match self {
            DocumentCreateTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: DocumentBaseTransition) {
        match self {
            DocumentCreateTransition::V0(v0) => v0.base = base,
        }
    }

    fn entropy(&self) -> [u8; 32] {
        match self {
            DocumentCreateTransition::V0(v0) => v0.entropy,
        }
    }

    fn set_entropy(&mut self, entropy: [u8; 32]) {
        match self {
            DocumentCreateTransition::V0(v0) => v0.entropy = entropy,
        }
    }

    fn created_at(&self) -> Option<TimestampMillis> {
        match self {
            DocumentCreateTransition::V0(v0) => v0.created_at,
        }
    }

    fn set_created_at(&mut self, created_at: Option<TimestampMillis>) {
        match self {
            DocumentCreateTransition::V0(v0) => v0.created_at = created_at,
        }
    }

    fn updated_at(&self) -> Option<TimestampMillis> {
        match self {
            DocumentCreateTransition::V0(v0) => v0.updated_at,
        }
    }

    fn set_updated_at(&mut self, updated_at: Option<TimestampMillis>) {
        match self {
            DocumentCreateTransition::V0(v0) => v0.updated_at = updated_at,
        }
    }

    fn data(&self) -> &BTreeMap<String, Value> {
        match self {
            DocumentCreateTransition::V0(v0) => &v0.data,
        }
    }

    fn data_mut(&mut self) -> &mut BTreeMap<String, Value> {
        match self {
            DocumentCreateTransition::V0(v0) => &mut v0.data,
        }
    }

    fn set_data(&mut self, data: BTreeMap<String, Value>) {
        match self {
            DocumentCreateTransition::V0(v0) => v0.data = data,
        }
    }
}
