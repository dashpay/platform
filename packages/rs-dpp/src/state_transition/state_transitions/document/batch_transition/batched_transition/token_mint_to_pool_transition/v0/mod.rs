pub mod v0_methods;

use crate::balances::credits::TokenAmount;
use crate::shielded::SerializedAction;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use bincode::{Decode, DecodeUntrusted, Encode};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use std::fmt;

/// The Identifier fields in [`TokenMintToPoolTransition`]
pub use super::super::document_base_transition::IDENTIFIER_FIELDS;

#[derive(Debug, Clone, Encode, Decode, PartialEq, DecodeUntrusted)]
// Auto-injects `json_safe_u64` on the u64 fields and `serde_bytes` on the fixed byte arrays /
// `serde_bytes_var` on `proof` (base64 strings in JSON, raw bytes in Value).
#[cfg_attr(feature = "json-conversion", crate::serialization::json_safe_fields)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct TokenMintToPoolTransitionV0 {
    /// Token base transition (nonce, contract, token position, token id).
    #[cfg_attr(feature = "serde-conversion", serde(flatten))]
    pub base: TokenBaseTransition,
    /// Tokens entering or leaving the pool. Equals the absolute value of the bundle's value balance.
    pub amount: TokenAmount,
    /// Orchard actions (spend-output pairs).
    pub actions: Vec<SerializedAction>,
    /// Sinsemilla root of the token pool's note commitment tree (Orchard anchor).
    pub anchor: [u8; 32],
    /// Halo 2 proof bytes.
    pub proof: Vec<u8>,
    /// RedPallas binding signature.
    pub binding_signature: [u8; 64],
    /// Optional public note. Only a group action proposer may set one.
    pub public_note: Option<String>,
}

impl Default for TokenMintToPoolTransitionV0 {
    fn default() -> Self {
        Self {
            base: TokenBaseTransition::default(),
            amount: 0,
            actions: vec![],
            anchor: [0u8; 32],
            proof: vec![],
            binding_signature: [0u8; 64],
            public_note: None,
        }
    }
}

impl fmt::Display for TokenMintToPoolTransitionV0 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Token Mint To Pool, base: {}, amount: {}, actions: {}",
            self.base,
            self.amount,
            self.actions.len()
        )
    }
}
