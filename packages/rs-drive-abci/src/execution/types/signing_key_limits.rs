use dpp::fee::Credits;
use dpp::identity::{KeyID, TimestampMillis};

/// The usage limits of the key that signed a state transition.
///
/// Identity signature validation fills this in when the signing key carries a budget or an
/// expiry, and the execution event carries it to fee validation and execution, the stages that
/// know the block time and what the state transition costs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SigningKeyLimits {
    /// The id of the key that signed the state transition
    pub key_id: KeyID,
    /// The block time, in milliseconds, from which the key can no longer sign
    pub expires_at: Option<TimestampMillis>,
    /// What was left of the key's budget when the signature was validated. `None` when the key
    /// has no budget.
    pub remaining_budget: Option<Credits>,
}
