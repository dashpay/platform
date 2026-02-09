use crate::error::Error;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use dpp::state_transition::{StateTransition, StateTransitionAddressEstimatedFeeValidation};
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use std::collections::BTreeMap;

/// A trait for validating minimum balance pre-checks for address-based state transitions.
pub(crate) trait StateTransitionAddressesMinimumBalanceValidationV0 {
    /// Validates that addresses have sufficient balance for the state transition including fees.
    ///
    /// This balance validation is not for the basic operations of the state transition,
    /// but as a quick early verification that the addresses have enough balance to cover
    /// the transfer/withdrawal amount plus any fees that may be required.
    ///
    /// # Arguments
    ///
    /// * `remaining_address_balances` - The remaining balances after the input amounts are consumed.
    /// * `platform_version` - The current platform version.
    ///
    /// # Returns
    ///
    /// * `Result<SimpleConsensusValidationResult, Error>` - A result indicating if the balance check passed.
    fn validate_addresses_minimum_balance_pre_check(
        &self,
        remaining_address_balances: &BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;

    /// True if the state transition has an addresses minimum balance pre-check validation.
    /// This balance validation is not for the operations of the state transition, but more as a
    /// quick early verification that the addresses have enough balance for the transfer/withdrawal plus fees.
    fn has_addresses_minimum_balance_pre_check_validation(&self) -> bool {
        true
    }
}

impl StateTransitionAddressesMinimumBalanceValidationV0 for StateTransition {
    fn validate_addresses_minimum_balance_pre_check(
        &self,
        remaining_address_balances: &BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        // Use the StateTransitionAddressEstimatedFeeValidation trait for address-based transitions
        let validation_result = match self {
            StateTransition::IdentityCreateFromAddresses(transition) => {
                transition.validate_estimated_fee(remaining_address_balances, platform_version)
            }
            StateTransition::IdentityTopUpFromAddresses(transition) => {
                transition.validate_estimated_fee(remaining_address_balances, platform_version)
            }
            StateTransition::AddressFundsTransfer(transition) => {
                transition.validate_estimated_fee(remaining_address_balances, platform_version)
            }
            StateTransition::AddressCreditWithdrawal(transition) => {
                transition.validate_estimated_fee(remaining_address_balances, platform_version)
            }
            // AddressFundingFromAssetLock doesn't need balance check - funds come from asset lock
            // Shielded transitions don't use address minimum balance validation
            // All other state transitions don't use address minimum balance validation
            StateTransition::AddressFundingFromAssetLock(_)
            | StateTransition::DataContractCreate(_)
            | StateTransition::DataContractUpdate(_)
            | StateTransition::IdentityCreate(_)
            | StateTransition::IdentityTopUp(_)
            | StateTransition::IdentityUpdate(_)
            | StateTransition::IdentityCreditTransfer(_)
            | StateTransition::IdentityCreditWithdrawal(_)
            | StateTransition::IdentityCreditTransferToAddresses(_)
            | StateTransition::Batch(_)
            | StateTransition::MasternodeVote(_)
            | StateTransition::Shield(_)
            | StateTransition::ShieldedTransfer(_)
            | StateTransition::Unshield(_) => {
                return Ok(SimpleConsensusValidationResult::new());
            }
        }?;

        // Convert ConsensusValidationResult<Credits> to SimpleConsensusValidationResult
        Ok(validation_result.map(|_| ()))
    }

    fn has_addresses_minimum_balance_pre_check_validation(&self) -> bool {
        match self {
            // Address-based transitions that need minimum balance validation for fees
            StateTransition::AddressFundsTransfer(_)
            | StateTransition::AddressCreditWithdrawal(_)
            | StateTransition::IdentityCreateFromAddresses(_)
            | StateTransition::IdentityTopUpFromAddresses(_) => true,
            // Identity-based transitions don't use address minimum balance validation
            StateTransition::DataContractCreate(_)
            | StateTransition::DataContractUpdate(_)
            | StateTransition::IdentityCreate(_)
            | StateTransition::IdentityTopUp(_)
            | StateTransition::IdentityUpdate(_)
            | StateTransition::IdentityCreditTransfer(_)
            | StateTransition::IdentityCreditWithdrawal(_)
            | StateTransition::IdentityCreditTransferToAddresses(_)
            | StateTransition::Batch(_)
            | StateTransition::MasternodeVote(_)
            | StateTransition::AddressFundingFromAssetLock(_)
            | StateTransition::Shield(_)
            | StateTransition::ShieldedTransfer(_)
            | StateTransition::Unshield(_) => false,
        }
    }
}
