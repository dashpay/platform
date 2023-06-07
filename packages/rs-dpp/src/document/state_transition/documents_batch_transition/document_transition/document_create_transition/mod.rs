mod v0;

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
