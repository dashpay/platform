pub mod v0;
mod v0_methods;
pub mod validate_structure;

use bincode::{Decode, Encode};
use derive_more::{Display, From};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
pub use v0::TokenUnfreezeTransitionV0;

#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
pub enum TokenUnfreezeTransition {
    #[display("V0({})", "_0")]
    V0(TokenUnfreezeTransitionV0),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for TokenUnfreezeTransition {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for TokenUnfreezeTransition {}

impl Default for TokenUnfreezeTransition {
    fn default() -> Self {
        TokenUnfreezeTransition::V0(TokenUnfreezeTransitionV0::default()) // since only v0
    }
}
