pub mod v0;
mod v0_methods;

use bincode::{Decode, Encode};
use derive_more::{Display, From};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};
pub use v0::TokenIssuanceTransitionV0;

#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize)
)]
pub enum TokenIssuanceTransition {
    #[display("V0({})", "_0")]
    V0(TokenIssuanceTransitionV0),
}

impl Default for TokenIssuanceTransition {
    fn default() -> Self {
        TokenIssuanceTransition::V0(TokenIssuanceTransitionV0::default()) // since only v0
    }
}
