mod v0;

use std::collections::BTreeMap;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use platform_value::Value;
pub use v0::*;
use crate::identity::TimestampMillis;
use crate::prelude::Revision;
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionAction;

#[cfg(feature = "state-transition-transformers")]
pub mod transformer;

#[derive(Debug, Clone)]
pub enum DocumentReplaceTransitionAction {
    V0(DocumentReplaceTransitionActionV0),
}

impl DocumentReplaceTransitionActionAccessorsV0 for DocumentReplaceTransitionAction {
    fn base(&self) -> &DocumentBaseTransitionAction {
        match self {
            DocumentReplaceTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> DocumentBaseTransitionAction {
        match self {
            DocumentReplaceTransitionAction::V0(v0) => v0.base,
        }
    }

    fn revision(&self) -> Revision {
        match self {
            DocumentReplaceTransitionAction::V0(v0) => v0.revision,
        }
    }

    fn created_at(&self) -> Option<TimestampMillis> {
        match self {
            DocumentReplaceTransitionAction::V0(v0) => v0.created_at,
        }
    }

    fn updated_at(&self) -> Option<TimestampMillis> {
        match self {
            DocumentReplaceTransitionAction::V0(v0) => v0.updated_at,
        }
    }

    fn data(&self) -> &BTreeMap<String, Value> {
        match self {
            DocumentReplaceTransitionAction::V0(v0) => &v0.data,
        }
    }

    fn data_owned(self) -> BTreeMap<String, Value> {
        match self {
            DocumentReplaceTransitionAction::V0(v0) => v0.data,
        }
    }
}
