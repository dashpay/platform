mod v0;

use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
use crate::state_transition::documents_batch_transition::document_transition::DocumentTransitionMethodsV0;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
pub use v0::*;

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode, PartialEq)]
pub enum DocumentCreateTransition {
    V0(DocumentCreateTransitionV0),
}

impl Default for DocumentCreateTransition {
    fn default() -> Self {
        DocumentCreateTransition::V0(DocumentCreateTransitionV0::default()) // since only v0
    }
}

impl DocumentTransitionMethodsV0 for DocumentCreateTransition {
    fn base(&self) -> &DocumentBaseTransition {
        match self {
            DocumentCreateTransition::V0(v0) => &v0.base,
        }
    }
}
