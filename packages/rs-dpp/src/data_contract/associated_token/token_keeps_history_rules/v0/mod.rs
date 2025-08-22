mod accessors;
use bincode::Encode;
use platform_serialization::de::Decode;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The rules for keeping a ledger as documents of token events.
/// Config update, Destroying Frozen Funds, Emergency Action,
/// Pre Programmed Token Release always require an entry to the ledger
#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TokenKeepsHistoryRulesV0 {
    /// Whether transfer history is recorded.
    #[serde(default = "default_true")]
    pub keeps_transfer_history: bool,

    /// Whether freezing history is recorded.
    #[serde(default = "default_true")]
    pub keeps_freezing_history: bool,

    /// Whether minting history is recorded.
    #[serde(default = "default_true")]
    pub keeps_minting_history: bool,

    /// Whether burning history is recorded.
    #[serde(default = "default_true")]
    pub keeps_burning_history: bool,

    /// Whether direct pricing history is recorded.
    #[serde(default = "default_true")]
    pub keeps_direct_pricing_history: bool,

    /// Whether direct purchase history is recorded.
    #[serde(default = "default_true")]
    pub keeps_direct_purchase_history: bool,
}

impl TokenKeepsHistoryRulesV0 {
    pub fn default_for_keeping_all_history(keeps_all_history: bool) -> TokenKeepsHistoryRulesV0 {
        TokenKeepsHistoryRulesV0 {
            keeps_transfer_history: keeps_all_history,
            keeps_freezing_history: keeps_all_history,
            keeps_minting_history: keeps_all_history,
            keeps_burning_history: keeps_all_history,
            keeps_direct_pricing_history: keeps_all_history,
            keeps_direct_purchase_history: keeps_all_history,
        }
    }
}

impl Default for TokenKeepsHistoryRulesV0 {
    fn default() -> Self {
        TokenKeepsHistoryRulesV0 {
            keeps_transfer_history: true,
            keeps_freezing_history: true,
            keeps_minting_history: true,
            keeps_burning_history: true,
            keeps_direct_pricing_history: true,
            keeps_direct_purchase_history: true,
        }
    }
}

/// Provides a default value of `true` for boolean fields.
fn default_true() -> bool {
    true
}

impl fmt::Display for TokenKeepsHistoryRulesV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TokenKeepsHistoryRulesV0 {{\n  keeps_transfer_history: {},\n  keeps_freezing_history: {},\n  keeps_minting_history: {},\n  keeps_burning_history: {},\n  keeps_direct_pricing_history: {}, \n keeps_direct_purchase_history: {}\n",
            self.keeps_transfer_history,
            self.keeps_freezing_history,
            self.keeps_minting_history,
            self.keeps_burning_history,
            self.keeps_direct_pricing_history,
            self.keeps_direct_purchase_history,
        )
    }
}
