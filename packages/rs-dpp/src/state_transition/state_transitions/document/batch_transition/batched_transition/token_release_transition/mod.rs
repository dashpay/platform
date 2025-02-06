pub mod v0;
mod v0_methods;
pub mod validate_structure;

use bincode::{Decode, Encode};
use derive_more::{Display, From};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};
pub use v0::TokenReleaseTransitionV0;

#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize)
)]
pub enum TokenReleaseTransition {
    #[display("V0({})", "_0")]
    V0(TokenReleaseTransitionV0),
}

impl Default for TokenReleaseTransition {
    fn default() -> Self {
        TokenReleaseTransition::V0(TokenReleaseTransitionV0::default()) // since only v0
    }
}
