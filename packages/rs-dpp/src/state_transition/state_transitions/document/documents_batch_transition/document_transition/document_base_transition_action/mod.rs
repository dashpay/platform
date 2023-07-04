
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
    }
}
