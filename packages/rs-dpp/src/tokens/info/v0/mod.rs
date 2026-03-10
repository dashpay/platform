#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use bincode::{Decode, Encode};
use derive_more::From;

#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Debug, Clone, Encode, Decode, From, PartialEq)]
#[cfg_attr(
    any(feature = "fixtures-and-mocks", feature = "serde-conversion"),
    derive(serde::Serialize, serde::Deserialize)
)]
/// Token information for an identity (version 0).
pub struct IdentityTokenInfoV0 {
    pub frozen: bool,
}

pub trait IdentityTokenInfoV0Accessors {
    /// Gets the frozen state of the identity.
    fn frozen(&self) -> bool;

    /// Sets the frozen state of the identity.
    fn set_frozen(&mut self, frozen: bool);
}

impl IdentityTokenInfoV0Accessors for IdentityTokenInfoV0 {
    fn frozen(&self) -> bool {
        self.frozen
    }

    fn set_frozen(&mut self, frozen: bool) {
        self.frozen = frozen;
    }
}
