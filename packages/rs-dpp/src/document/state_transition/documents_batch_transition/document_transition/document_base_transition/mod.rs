mod v0;

pub use v0::*;
use serde::{Deserialize, Serialize};
use bincode::{Decode, Encode};

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode, Default, PartialEq)]
pub enum DocumentBaseTransition {
    V0(DocumentBaseTransitionV0)
}