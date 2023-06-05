mod v0;

use crate::version::{LATEST_PLATFORM_VERSION, PLATFORM_VERSIONS};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
pub use v0::*;

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode, PartialEq)]
pub enum DocumentBaseTransition {
    V0(DocumentBaseTransitionV0),
}

impl Default for DocumentBaseTransition {
    fn default() -> Self {
        DocumentBaseTransition::V0(DocumentBaseTransitionV0::default()) // since only v0
    }
}
