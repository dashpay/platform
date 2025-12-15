use crate::address_funds::PlatformAddress;
use crate::consensus::state::address_funds::AddressesNotEnoughFundsError;
use crate::fee::Credits;
use crate::prelude::AddressNonce;
use crate::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

/// Trait for estimating fees for state transitions.
///
/// This trait provides a method to calculate estimated fees based on the
/// transition's characteristics (inputs, outputs, etc.).
pub trait StateTransitionEstimatedFeeValidation {
    /// Calculates the estimated minimum fee required for this state transition.
    ///
    /// The fee is calculated based on the number of inputs, outputs, and any
    /// transition-specific costs (e.g., key creation costs for identity creation).
    ///
    /// # Arguments
    ///
    /// * `platform_version` - The platform version containing fee configuration.
    ///
    /// # Returns
    ///
    /// The estimated fee in credits.
    fn calculate_estimated_fee(&self, platform_version: &PlatformVersion) -> Credits;
}

/// Trait for validating that address-based state transitions have sufficient funds for fees.
///
/// This trait extends fee estimation with validation capabilities specific to
/// address-based state transitions that pay fees from address balances rather
/// than identity balances.
pub trait StateTransitionAddressEstimatedFeeValidation:
    StateTransitionEstimatedFeeValidation
{
    /// Validates that sufficient funds are available to cover the estimated fee.
    ///
    /// This is a pre-check validation to quickly verify that the addresses
    /// referenced in the fee strategy have enough remaining balance to pay
    /// the estimated fee.
    ///
    /// # Arguments
    ///
    /// * `remaining_balances` - The remaining balances of addresses after input amounts are consumed.
    /// * `platform_version` - The platform version containing fee configuration.
    ///
    /// # Returns
    ///
    /// A validation result. If validation fails, contains an `AddressesNotEnoughFundsError`.
    fn validate_estimated_fee(
        &self,
        remaining_balances: &BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        platform_version: &PlatformVersion,
    ) -> ConsensusValidationResult<Credits> {
        let required_fee = self.calculate_estimated_fee(platform_version);
        let amount_available = self.calculate_amount_available(remaining_balances);

        if amount_available < required_fee {
            ConsensusValidationResult::new_with_error(
                AddressesNotEnoughFundsError::new(remaining_balances.clone(), required_fee).into(),
            )
        } else {
            ConsensusValidationResult::new()
        }
    }

    /// Calculates the total amount available for fee payment based on the fee strategy.
    ///
    /// This examines the transition's fee strategy and sums up the available credits
    /// from inputs (via `DeductFromInput`) and outputs (via `ReduceOutput`) that are
    /// designated for fee payment.
    ///
    /// # Arguments
    ///
    /// * `remaining_balances` - The remaining balances of addresses after input amounts are consumed.
    ///
    /// # Returns
    ///
    /// The total credits available for fee payment.
    fn calculate_amount_available(
        &self,
        remaining_balances: &BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    ) -> Credits;
}

/// Trait for validating that identity-based state transitions have sufficient funds for fees.
///
/// This trait extends fee estimation with validation capabilities specific to
/// identity-based state transitions that pay fees from identity balances.
pub trait StateTransitionIdentityEstimatedFeeValidation:
    StateTransitionEstimatedFeeValidation
{
    /// Validates that sufficient identity balance is available to cover the estimated fee.
    ///
    /// # Arguments
    ///
    /// * `identity_known_balance` - The known balance of the identity.
    /// * `platform_version` - The platform version containing fee configuration.
    ///
    /// # Returns
    ///
    /// A validation result. If validation fails, contains an `IdentityInsufficientBalanceError`.
    fn validate_estimated_fee(
        &self,
        identity_known_balance: Credits,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult;
}
