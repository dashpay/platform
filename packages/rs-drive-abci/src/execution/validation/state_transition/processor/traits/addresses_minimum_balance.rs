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
            | StateTransition::Unshield(_)
            | StateTransition::ShieldFromAssetLock(_)
            | StateTransition::ShieldedWithdrawal(_)
            | StateTransition::IdentityCreateFromShieldedPool(_) => {
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
            | StateTransition::Unshield(_)
            | StateTransition::ShieldFromAssetLock(_)
            | StateTransition::ShieldedWithdrawal(_)
            | StateTransition::IdentityCreateFromShieldedPool(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn make_data_contract_create_st() -> StateTransition {
        use dpp::tests::fixtures::get_data_contract_fixture;
        use platform_version::TryIntoPlatformVersioned;
        let platform_version = platform_version::version::PlatformVersion::latest();
        let created_data_contract =
            get_data_contract_fixture(None, 1, platform_version.protocol_version);
        let transition: dpp::state_transition::data_contract_create_transition::DataContractCreateTransition =
            created_data_contract.try_into_platform_versioned(platform_version).unwrap();
        transition.into()
    }

    use dpp::state_transition::batch_transition::BatchTransition;
    use dpp::state_transition::batch_transition::BatchTransitionV0;
    use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
    use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
    use dpp::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
    use dpp::state_transition::state_transitions::identity::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
    use dpp::state_transition::state_transitions::identity::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;
    use dpp::state_transition::state_transitions::identity::identity_topup_from_addresses_transition::IdentityTopUpFromAddressesTransition;
    use dpp::state_transition::state_transitions::identity::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0;
    use dpp::state_transition::state_transitions::address_funds::address_funds_transfer_transition::AddressFundsTransferTransition;
    use dpp::state_transition::state_transitions::address_funds::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
    use dpp::state_transition::state_transitions::address_funds::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
    use dpp::state_transition::state_transitions::address_funds::address_funding_from_asset_lock_transition::v0::AddressFundingFromAssetLockTransitionV0;
    use dpp::state_transition::state_transitions::address_funds::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition;
    use dpp::state_transition::state_transitions::address_funds::address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0;

    mod has_addresses_minimum_balance_pre_check_validation {
        use super::*;

        #[test]
        fn should_return_true_for_address_based_transfers_and_withdrawals() {
            let transitions: Vec<(&str, StateTransition)> = vec![
                (
                    "AddressFundsTransfer",
                    StateTransition::AddressFundsTransfer(AddressFundsTransferTransition::V0(
                        AddressFundsTransferTransitionV0::default(),
                    )),
                ),
                (
                    "AddressCreditWithdrawal",
                    StateTransition::AddressCreditWithdrawal(
                        AddressCreditWithdrawalTransition::V0(
                            AddressCreditWithdrawalTransitionV0::default(),
                        ),
                    ),
                ),
                (
                    "IdentityCreateFromAddresses",
                    StateTransition::IdentityCreateFromAddresses(
                        IdentityCreateFromAddressesTransition::V0(
                            IdentityCreateFromAddressesTransitionV0::default(),
                        ),
                    ),
                ),
                (
                    "IdentityTopUpFromAddresses",
                    StateTransition::IdentityTopUpFromAddresses(
                        IdentityTopUpFromAddressesTransition::V0(
                            IdentityTopUpFromAddressesTransitionV0::default(),
                        ),
                    ),
                ),
            ];
            for (name, st) in transitions {
                assert!(
                    st.has_addresses_minimum_balance_pre_check_validation(),
                    "expected true for {}",
                    name
                );
            }
        }

        #[test]
        fn should_return_false_for_identity_based_and_other_transitions() {
            let transitions: Vec<(&str, StateTransition)> = vec![
                ("DataContractCreate", make_data_contract_create_st()),
                (
                    "Batch",
                    StateTransition::Batch(BatchTransition::V0(BatchTransitionV0::default())),
                ),
                (
                    "IdentityCreate",
                    StateTransition::IdentityCreate(IdentityCreateTransition::V0(
                        IdentityCreateTransitionV0::default(),
                    )),
                ),
                (
                    "AddressFundingFromAssetLock",
                    StateTransition::AddressFundingFromAssetLock(
                        AddressFundingFromAssetLockTransition::V0(
                            AddressFundingFromAssetLockTransitionV0::default(),
                        ),
                    ),
                ),
            ];
            for (name, st) in transitions {
                assert!(
                    !st.has_addresses_minimum_balance_pre_check_validation(),
                    "expected false for {}",
                    name
                );
            }
        }
    }

    mod validate_addresses_minimum_balance_pre_check {
        use super::*;

        #[test]
        fn should_return_valid_for_non_address_transition() {
            let st = make_data_contract_create_st();
            let platform_version = &platform_version::version::v1::PLATFORM_V1;
            let balances = BTreeMap::new();
            let result = st
                .validate_addresses_minimum_balance_pre_check(&balances, platform_version)
                .expect("should not error");
            assert!(result.is_valid());
        }
    }
}
