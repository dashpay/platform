mod v0;

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
pub use v0::*;

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode, PartialEq)]
pub enum DocumentDeleteTransition {
    V0(DocumentDeleteTransitionV0),
}
