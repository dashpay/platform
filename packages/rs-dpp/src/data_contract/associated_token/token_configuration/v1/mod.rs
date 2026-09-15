mod accessors;

use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use bincode::{Decode, DecodeUntrusted, Encode};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Version 1 of the token configuration: everything `TokenConfigurationV0` carries, plus the
/// per-token shielded pool opt-in.
///
/// The V0 fields are nested as `base` and flattened on the JSON / Value wire, so a V1
/// configuration reads exactly like a V0 one with `$formatVersion: "1"` and the extra
/// `hasShieldedPool` key. On the bincode wire it is the V0 bytes followed by the flag, under
/// the enum's variant index 1, so stored V0 configurations decode unchanged.
///
/// `has_shielded_pool` is decided at token creation and is immutable afterwards: the pool
/// subtree (an Orchard note commitment tree, nullifier set, anchors and a balance) is created
/// together with the token's other trees, and a pool holding notes can never be removed.
/// Enabling a pool on an existing token is not supported by this version.
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, DecodeUntrusted)]
#[serde(rename_all = "camelCase")]
pub struct TokenConfigurationV1 {
    /// The V0 configuration this version extends.
    #[serde(flatten)]
    pub base: TokenConfigurationV0,
    /// Whether this token has its own shielded pool. When `true`, holders may move balance
    /// into the pool (`TokenShield`), transfer privately inside it (`TokenShieldedTransfer`)
    /// and move balance back out to an identity (`TokenUnshield`).
    #[serde(default)]
    pub has_shielded_pool: bool,
}

impl TokenConfigurationV1 {
    /// Wraps a V0 configuration, opting the token into a shielded pool when `has_shielded_pool`.
    pub fn from_v0(base: TokenConfigurationV0, has_shielded_pool: bool) -> Self {
        Self {
            base,
            has_shielded_pool,
        }
    }
}

impl fmt::Display for TokenConfigurationV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TokenConfigurationV1 {{\n  base: {},\n  has_shielded_pool: {}\n}}",
            self.base, self.has_shielded_pool
        )
    }
}
