use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::identity::PartialIdentity;
use dpp::state_transition::{StateTransition, StateTransitionIdentityEstimatedFeeValidation};
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

/// A trait for validating state transitions within a blockchain.
pub(crate) trait StateTransitionIdentityBalanceValidationV0 {
    /// Validates the state transition by analyzing the changes in the platform state after applying the transaction.
    ///
    /// # Arguments
    ///
    /// * `platform` - A reference to the platform containing the state data.
    /// * `tx` - The transaction argument to be applied.
    ///
    /// # Type Parameters
    ///
    /// * `C: CoreRPCLike` - A type constraint indicating that C should implement `CoreRPCLike`.
    ///
    /// # Returns
    ///
    /// * `Result<ConsensusValidationResult<StateTransitionAction>, Error>` - A result with either a ConsensusValidationResult containing a StateTransitionAction or an Error.
    fn validate_identity_minimum_balance_pre_check(
        &self,
        identity: &PartialIdentity,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;

    /// True if the state transition has a balance validation.
    /// This balance validation is not for the operations of the state transition, but more as a
    /// quick early verification that the user has the balance they want to transfer or withdraw.
    fn has_identity_minimum_balance_pre_check_validation(&self) -> bool {
        true
    }
}

impl StateTransitionIdentityBalanceValidationV0 for StateTransition {
    fn validate_identity_minimum_balance_pre_check(
        &self,
        identity: &PartialIdentity,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let balance =
            identity
                .balance
                .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "expected to have a balance on identity for credit transfer transition",
                )))?;
        match self {
            StateTransition::IdentityCreditTransfer(st) => st
                .validate_estimated_fee(balance, platform_version)
                .map_err(Error::Protocol),
            StateTransition::IdentityCreditWithdrawal(st) => st
                .validate_estimated_fee(balance, platform_version)
                .map_err(Error::Protocol),
            StateTransition::Batch(st) => st
                .validate_estimated_fee(balance, platform_version)
                .map_err(Error::Protocol),
            StateTransition::IdentityCreditTransferToAddresses(st) => st
                .validate_estimated_fee(balance, platform_version)
                .map_err(Error::Protocol),
            StateTransition::DataContractCreate(st) => st
                .validate_estimated_fee(balance, platform_version)
                .map_err(Error::Protocol),
            StateTransition::DataContractUpdate(st) => st
                .validate_estimated_fee(balance, platform_version)
                .map_err(Error::Protocol),
            StateTransition::IdentityUpdate(st) => st
                .validate_estimated_fee(balance, platform_version)
                .map_err(Error::Protocol),
            StateTransition::MasternodeVote(_)
            | StateTransition::IdentityCreate(_)
            | StateTransition::IdentityTopUp(_)
            | StateTransition::IdentityCreateFromAddresses(_)
            | StateTransition::IdentityTopUpFromAddresses(_)
            | StateTransition::AddressFundsTransfer(_)
            | StateTransition::AddressFundingFromAssetLock(_)
            | StateTransition::AddressCreditWithdrawal(_)
            | StateTransition::Shield(_)
            | StateTransition::ShieldedTransfer(_)
            | StateTransition::Unshield(_)
            | StateTransition::ShieldFromAssetLock(_)
            | StateTransition::ShieldedWithdrawal(_)
            | StateTransition::IdentityCreateFromShieldedPool(_) => {
                Ok(SimpleConsensusValidationResult::new())
            }
        }
    }

    fn has_identity_minimum_balance_pre_check_validation(&self) -> bool {
        matches!(
            self,
            StateTransition::IdentityCreditTransfer(_)
                | StateTransition::IdentityCreditWithdrawal(_)
                | StateTransition::DataContractCreate(_)
                | StateTransition::DataContractUpdate(_)
                | StateTransition::Batch(_)
                | StateTransition::IdentityUpdate(_)
                | StateTransition::IdentityCreditTransferToAddresses(_)
        )
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

    fn make_data_contract_update_st() -> StateTransition {
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::tests::fixtures::get_data_contract_fixture;
        use platform_version::TryIntoPlatformVersioned;
        let platform_version = platform_version::version::PlatformVersion::latest();
        let created_data_contract =
            get_data_contract_fixture(None, 1, platform_version.protocol_version);
        let data_contract = created_data_contract.data_contract().clone();
        let transition: dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition =
            (data_contract, 2u64).try_into_platform_versioned(platform_version).unwrap();
        transition.into()
    }

    use dpp::state_transition::batch_transition::BatchTransition;
    use dpp::state_transition::batch_transition::BatchTransitionV0;
    use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
    use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
    use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
    use dpp::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
    use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
    use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
    use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
    use dpp::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;
    use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
    use dpp::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
    use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
    use dpp::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
    use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
    use dpp::state_transition::masternode_vote_transition::v0::MasternodeVoteTransitionV0;
    use dpp::state_transition::state_transitions::identity::identity_credit_transfer_to_addresses_transition::IdentityCreditTransferToAddressesTransition;
    use dpp::state_transition::state_transitions::identity::identity_credit_transfer_to_addresses_transition::v0::IdentityCreditTransferToAddressesTransitionV0;

    mod has_identity_minimum_balance_pre_check_validation {
        use super::*;

        #[test]
        fn should_return_true_for_transitions_with_balance_pre_check() {
            let transitions: Vec<(&str, StateTransition)> = vec![
                (
                    "IdentityCreditTransfer",
                    StateTransition::IdentityCreditTransfer(IdentityCreditTransferTransition::V0(
                        IdentityCreditTransferTransitionV0::default(),
                    )),
                ),
                (
                    "IdentityCreditWithdrawal",
                    StateTransition::IdentityCreditWithdrawal(
                        IdentityCreditWithdrawalTransition::V0(
                            IdentityCreditWithdrawalTransitionV0::default(),
                        ),
                    ),
                ),
                ("DataContractCreate", make_data_contract_create_st()),
                ("DataContractUpdate", make_data_contract_update_st()),
                (
                    "Batch",
                    StateTransition::Batch(BatchTransition::V0(BatchTransitionV0::default())),
                ),
                (
                    "IdentityUpdate",
                    StateTransition::IdentityUpdate(IdentityUpdateTransition::V0(
                        IdentityUpdateTransitionV0::default(),
                    )),
                ),
                (
                    "IdentityCreditTransferToAddresses",
                    StateTransition::IdentityCreditTransferToAddresses(
                        IdentityCreditTransferToAddressesTransition::V0(
                            IdentityCreditTransferToAddressesTransitionV0::default(),
                        ),
                    ),
                ),
            ];
            for (name, st) in transitions {
                assert!(
                    st.has_identity_minimum_balance_pre_check_validation(),
                    "expected true for {}",
                    name
                );
            }
        }

        #[test]
        fn should_return_false_for_transitions_without_balance_pre_check() {
            let transitions: Vec<(&str, StateTransition)> = vec![
                (
                    "MasternodeVote",
                    StateTransition::MasternodeVote(MasternodeVoteTransition::V0(
                        MasternodeVoteTransitionV0::default(),
                    )),
                ),
                (
                    "IdentityCreate",
                    StateTransition::IdentityCreate(IdentityCreateTransition::V0(
                        IdentityCreateTransitionV0::default(),
                    )),
                ),
                (
                    "IdentityTopUp",
                    StateTransition::IdentityTopUp(IdentityTopUpTransition::V0(
                        IdentityTopUpTransitionV0::default(),
                    )),
                ),
                {
                    use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::v0::IdentityCreateFromShieldedPoolTransitionV0;
                    use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition;
                    (
                        "IdentityCreateFromShieldedPool",
                        StateTransition::IdentityCreateFromShieldedPool(
                            IdentityCreateFromShieldedPoolTransition::V0(
                                IdentityCreateFromShieldedPoolTransitionV0 {
                                    public_keys: vec![],
                                    denomination: 0,
                                    actions: vec![],
                                    anchor: [0u8; 32],
                                    proof: vec![],
                                    binding_signature: [0u8; 64],
                                    send_to_address_on_creation_failure:
                                        dpp::address_funds::PlatformAddress::P2pkh([0u8; 20]),
                                    identity_id: Default::default(),
                                },
                            ),
                        ),
                    )
                },
            ];
            for (name, st) in transitions {
                assert!(
                    !st.has_identity_minimum_balance_pre_check_validation(),
                    "expected false for {}",
                    name
                );
            }
        }
    }

    mod validate_identity_minimum_balance_pre_check {
        use super::*;

        #[test]
        fn should_return_error_when_identity_has_no_balance() {
            let identity = PartialIdentity {
                id: Default::default(),
                loaded_public_keys: Default::default(),
                balance: None,
                revision: None,
                not_found_public_keys: Default::default(),
            };
            let st = StateTransition::IdentityCreditTransfer(IdentityCreditTransferTransition::V0(
                IdentityCreditTransferTransitionV0::default(),
            ));
            let platform_version = &platform_version::version::v1::PLATFORM_V1;
            let result =
                st.validate_identity_minimum_balance_pre_check(&identity, platform_version);
            assert!(result.is_err(), "expected error when balance is None");
        }

        #[test]
        fn should_return_ok_for_transitions_without_balance_check() {
            let identity = PartialIdentity {
                id: Default::default(),
                loaded_public_keys: Default::default(),
                balance: Some(0),
                revision: None,
                not_found_public_keys: Default::default(),
            };
            let st = StateTransition::MasternodeVote(MasternodeVoteTransition::V0(
                MasternodeVoteTransitionV0::default(),
            ));
            let platform_version = &platform_version::version::v1::PLATFORM_V1;
            let result = st
                .validate_identity_minimum_balance_pre_check(&identity, platform_version)
                .expect("should not error");
            assert!(result.is_valid());
        }
    }
}
