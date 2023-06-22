use crate::document::document_transition::document_base_transition_action::v0::DocumentBaseTransitionActionV0;

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
mod v0;

pub use v0::*;

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode, PartialEq)]
pub enum DocumentBaseTransitionAction {
    V0(DocumentBaseTransitionActionV0),
}

impl Default for DocumentBaseTransitionAction {
    fn default() -> Self {
        DocumentBaseTransitionAction::V0(DocumentBaseTransitionActionV0::default())
        // since only v0
    }
}
