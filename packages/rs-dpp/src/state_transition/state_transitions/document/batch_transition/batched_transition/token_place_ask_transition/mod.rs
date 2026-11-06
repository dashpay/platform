pub mod v0;
mod v0_methods;
pub mod validate_structure;

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
use crate::serialization::JsonConvertible;
#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
use crate::serialization::ValueConvertible;
use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::{Display, From};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
pub use v0::TokenPlaceAskTransitionV0;

/// A versioned `TokenPlaceAskTransition` (protocol version 15, the token marketplace).
#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From, DecodeUntrusted)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
pub enum TokenPlaceAskTransition {
    #[display("V0({})", "_0")]
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(TokenPlaceAskTransitionV0),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl JsonConvertible for TokenPlaceAskTransition {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl ValueConvertible for TokenPlaceAskTransition {}

impl Default for TokenPlaceAskTransition {
    fn default() -> Self {
        TokenPlaceAskTransition::V0(TokenPlaceAskTransitionV0::default()) // since only v0
    }
}
