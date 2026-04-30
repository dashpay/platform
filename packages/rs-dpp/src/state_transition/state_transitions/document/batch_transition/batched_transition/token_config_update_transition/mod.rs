pub mod v0;
mod v0_methods;
pub mod validate_structure;

use bincode::{Decode, Encode};
use derive_more::{Display, From};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
pub use v0::TokenConfigUpdateTransitionV0;

#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
pub enum TokenConfigUpdateTransition {
    #[display("V0({})", "_0")]
    V0(TokenConfigUpdateTransitionV0),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for TokenConfigUpdateTransition {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for TokenConfigUpdateTransition {}

impl Default for TokenConfigUpdateTransition {
    fn default() -> Self {
        TokenConfigUpdateTransition::V0(TokenConfigUpdateTransitionV0::default())
        // since only v0
    }
}
