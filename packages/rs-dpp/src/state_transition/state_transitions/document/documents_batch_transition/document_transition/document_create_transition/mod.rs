pub mod from_document;
pub(crate) mod v0;
mod v0_methods;

use bincode::{Decode, Encode};
use derive_more::{Display, From};
use serde::{Deserialize, Serialize};
pub use v0::*;

#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize)
)]
pub enum DocumentCreateTransition {
    #[display(fmt = "V0({})", "_0")]
    V0(DocumentCreateTransitionV0),
}

impl Default for DocumentCreateTransition {
    fn default() -> Self {
        DocumentCreateTransition::V0(DocumentCreateTransitionV0::default()) // since only v0
    }
}
