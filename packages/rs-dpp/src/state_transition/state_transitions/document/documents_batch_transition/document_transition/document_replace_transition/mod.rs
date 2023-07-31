pub mod v0;
mod v0_methods;
use bincode::{Decode, Encode};
use derive_more::{Display, From};
use serde::{Deserialize, Serialize};
pub use v0::*;

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode, PartialEq, Display, From)]
pub enum DocumentReplaceTransition {
    #[display(fmt = "V0({})", "_0")]
    V0(DocumentReplaceTransitionV0),
}
