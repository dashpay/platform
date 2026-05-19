use crate::prelude::UserFeeIncrease;

/// Trait for state transitions that support a user-adjustable fee increase.
///
/// Not all state transitions support fee adjustment — for example, shielded
/// transitions have fees cryptographically locked by the Orchard binding
/// signature, and masternode votes always use a fee increase of 0.
pub trait StateTransitionHasUserFeeIncrease {
    /// returns the fee multiplier
    fn user_fee_increase(&self) -> UserFeeIncrease;
    /// set a fee multiplier
    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease);
}
