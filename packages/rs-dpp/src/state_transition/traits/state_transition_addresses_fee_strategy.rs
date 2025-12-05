use crate::address_funds::AddressFundsFeeStrategy;

/// Trait for state transitions that use an address-based fee strategy.
///
/// This trait provides access to the fee strategy for state transitions that
/// deduct fees from input addresses or reduce output amounts.
pub trait StateTransitionAddressesFeeStrategy {
    /// Get the fee strategy for this state transition.
    ///
    /// The fee strategy defines how fees should be deducted from inputs
    /// or outputs when processing the state transition.
    fn fee_strategy(&self) -> &AddressFundsFeeStrategy;

    /// Set the fee strategy for this state transition.
    fn set_fee_strategy(&mut self, fee_strategy: AddressFundsFeeStrategy);
}
