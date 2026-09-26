mod accessors;

use crate::data_contract::associated_token::token_configuration::v0::{
    default_change_control_rules, TokenConfigurationV0,
};
use crate::data_contract::change_control_rules::ChangeControlRules;
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use bincode::{Decode, DecodeUntrusted, Encode};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Version 1 of the token configuration: everything `TokenConfigurationV0` carries, plus the
/// per-token shielded pool opt-in and the pool's outgoing notes threshold.
///
/// The V0 fields are nested as `base` and flattened on the JSON / Value wire, so a V1
/// configuration reads exactly like a V0 one with `$formatVersion: "1"` and the extra
/// `hasShieldedPool`, `minimumPoolNotesForOutgoing` and `minimumPoolNotesForOutgoingChangeRules`
/// keys. On the bincode wire it is the V0 bytes followed by the flag, the threshold and its
/// rules, under the enum's variant index 1, so stored V0 configurations decode unchanged.
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
    /// The fewest notes the token's shielded pool must hold before tokens may leave it for a
    /// visible destination (unshielding to an identity, burning from the pool, paying a
    /// document's token cost from the pool). Transfers inside the pool are not limited.
    /// `None` reads as 0, no threshold.
    ///
    /// The count is of note commitments, not of holders: one bundle carries several actions,
    /// so a single depositor can reach a threshold alone. It tells holders how busy the pool
    /// should be before they leave it and guarantees no anonymity set. A threshold above 0
    /// also traps a new pool's first depositors until enough notes accumulate, so the default
    /// is none. At most `max_token_pool_notes_for_outgoing`, so an issuer cannot set one no
    /// pool reaches and strand every shielded balance. Changed through `TokenConfigUpdate`
    /// under `minimum_pool_notes_for_outgoing_change_rules`.
    #[serde(default)]
    pub minimum_pool_notes_for_outgoing: Option<u64>,
    /// Change control rules governing who can modify `minimum_pool_notes_for_outgoing`. No one
    /// when absent.
    #[serde(default = "default_change_control_rules")]
    pub minimum_pool_notes_for_outgoing_change_rules: ChangeControlRules,
}

impl TokenConfigurationV1 {
    /// Wraps a V0 configuration, opting the token into a shielded pool when `has_shielded_pool`.
    pub fn from_v0(base: TokenConfigurationV0, has_shielded_pool: bool) -> Self {
        Self {
            base,
            has_shielded_pool,
            minimum_pool_notes_for_outgoing: None,
            minimum_pool_notes_for_outgoing_change_rules: default_change_control_rules(),
        }
    }
}

impl fmt::Display for TokenConfigurationV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TokenConfigurationV1 {{\n  base: {},\n  has_shielded_pool: {},\n  minimum_pool_notes_for_outgoing: {:?},\n  minimum_pool_notes_for_outgoing_change_rules: {:?}\n}}",
            self.base,
            self.has_shielded_pool,
            self.minimum_pool_notes_for_outgoing,
            self.minimum_pool_notes_for_outgoing_change_rules
        )
    }
}
