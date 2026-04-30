pub mod v0;
mod v0_methods;
pub mod validate_structure;

use bincode::{Decode, Encode};
use derive_more::{Display, From};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
pub use v0::TokenClaimTransitionV0;

#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
pub enum TokenClaimTransition {
    #[display("V0({})", "_0")]
    V0(TokenClaimTransitionV0),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for TokenClaimTransition {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for TokenClaimTransition {}

impl Default for TokenClaimTransition {
    fn default() -> Self {
        TokenClaimTransition::V0(TokenClaimTransitionV0::default()) // since only v0
    }
}
