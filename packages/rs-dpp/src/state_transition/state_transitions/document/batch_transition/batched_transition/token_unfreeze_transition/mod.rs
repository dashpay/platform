pub mod v0;
mod v0_methods;

use bincode::{Decode, Encode};
use derive_more::{Display, From};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};
pub use v0::TokenUnfreezeTransitionV0;

#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize)
)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub enum TokenUnfreezeTransition {
    #[display("V0({})", "_0")]
    V0(TokenUnfreezeTransitionV0),
}

impl Default for TokenUnfreezeTransition {
    fn default() -> Self {
        TokenUnfreezeTransition::V0(TokenUnfreezeTransitionV0::default()) // since only v0
    }
}
