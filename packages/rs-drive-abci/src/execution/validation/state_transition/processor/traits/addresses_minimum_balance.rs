use crate::error::Error;
use dpp::address_funds::PlatformAddress;
use dpp::consensus::state::address_funds::AddressesNotEnoughFundsError;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use dpp::state_transition::address_credit_withdrawal_transition::accessors::AddressCreditWithdrawalTransitionAccessorsV0;
use dpp::state_transition::address_funds_transfer_transition::accessors::AddressFundsTransferTransitionAccessorsV0;
use dpp::state_transition::identity_create_from_addresses_transition::accessors::IdentityCreateFromAddressesTransitionAccessorsV0;
use dpp::state_transition::identity_topup_from_addresses_transition::accessors::IdentityTopUpFromAddressesTransitionAccessorsV0;
use dpp::state_transition::StateTransition;
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
        let min_fees = &platform_version.fee_version.state_transition_min_fees;

        // Calculate the required fee based on the transition type
        let required_fee = match self {
            StateTransition::IdentityCreateFromAddresses(transition) => {
                let input_count = match transition {
                    dpp::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition::V0(v0) => v0.inputs.len(),
                };
                let output_count = if transition.output().is_some() { 1 } else { 0 };
                min_fees
                    .identity_create_from_addresses
                    .saturating_add(
                        min_fees
                            .address_funds_transfer_input_cost
                            .saturating_mul(input_count as u64),
                    )
                    .saturating_add(
                        min_fees
                            .address_funds_transfer_output_cost
                            .saturating_mul(output_count),
                    )
            }
            StateTransition::IdentityTopUpFromAddresses(transition) => {
                let input_count = match transition {
                    dpp::state_transition::identity_topup_from_addresses_transition::IdentityTopUpFromAddressesTransition::V0(v0) => v0.inputs.len(),
                };
                let output_count = if transition.output().is_some() { 1 } else { 0 };
                min_fees
                    .identity_topup_from_addresses
                    .saturating_add(
                        min_fees
                            .address_funds_transfer_input_cost
                            .saturating_mul(input_count as u64),
                    )
                    .saturating_add(
                        min_fees
                            .address_funds_transfer_output_cost
                            .saturating_mul(output_count),
                    )
            }
            StateTransition::AddressFundsTransfer(transition) => {
                let input_count = match transition {
                    dpp::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition::V0(v0) => v0.inputs.len(),
                };
                let output_count = transition.outputs().len().max(1);
                min_fees
                    .address_funds_transfer_input_cost
                    .saturating_mul(input_count as u64)
                    .saturating_add(
                        min_fees
                            .address_funds_transfer_output_cost
                            .saturating_mul(output_count as u64),
                    )
            }
            StateTransition::AddressCreditWithdrawal(transition) => {
                let input_count = match transition {
                    dpp::state_transition::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition::V0(v0) => v0.inputs.len(),
                };
                let output_count = if transition.output().is_some() { 1 } else { 0 };
                min_fees
                    .address_credit_withdrawal
                    .saturating_add(
                        min_fees
                            .address_funds_transfer_input_cost
                            .saturating_mul(input_count as u64),
                    )
                    .saturating_add(
                        min_fees
                            .address_funds_transfer_output_cost
                            .saturating_mul(output_count),
                    )
            }
            // AddressFundingFromAssetLock doesn't need balance check - funds come from asset lock
            StateTransition::AddressFundingFromAssetLock(_) => {
                return Ok(SimpleConsensusValidationResult::new());
            }
            // All other state transitions don't use address minimum balance validation
            StateTransition::DataContractCreate(_)
            | StateTransition::DataContractUpdate(_)
            | StateTransition::IdentityCreate(_)
            | StateTransition::IdentityTopUp(_)
            | StateTransition::IdentityUpdate(_)
            | StateTransition::IdentityCreditTransfer(_)
            | StateTransition::IdentityCreditWithdrawal(_)
            | StateTransition::IdentityCreditTransferToAddresses(_)
            | StateTransition::Batch(_)
            | StateTransition::MasternodeVote(_) => {
                return Ok(SimpleConsensusValidationResult::new());
            }
        };

        // Sum all remaining balances
        let total_remaining: Credits = remaining_address_balances
            .values()
            .map(|(_, credits)| *credits)
            .sum();

        if total_remaining < required_fee {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                AddressesNotEnoughFundsError::new(remaining_address_balances.clone(), required_fee)
                    .into(),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }

    fn has_addresses_minimum_balance_pre_check_validation(&self) -> bool {
        match self {
            // Address-based transitions that need minimum balance validation for fees
            StateTransition::AddressFundsTransfer(_)
            | StateTransition::AddressCreditWithdrawal(_)
            | StateTransition::IdentityCreateFromAddresses(_)
            | StateTransition::IdentityTopUpFromAddresses(_)
            | StateTransition::AddressFundingFromAssetLock(_) => true,
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
            | StateTransition::MasternodeVote(_) => false,
        }
    }
}
