use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::platform_types::platform::PlatformStateRef;
use dpp::block::block_info::BlockInfo;
use dpp::state_transition::StateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

/// A trait for validating identity nonce rules within a state transition.
pub(crate) trait StateTransitionIdentityNonceValidationV0 {
    /// Validates the identity nonce constraints for this state transition.
    ///
    /// # Arguments
    ///
    /// * `platform` – Reference to the platform state.
    /// * `block_info` – Information about the current block.
    /// * `tx` – The raw transaction argument.
    /// * `execution_context` – Execution context for the state transition.
    /// * `platform_version` – The active platform version.
    ///
    /// # Returns
    ///
    /// Returns a [`SimpleConsensusValidationResult`] on success
    /// or an [`Error`] if validation fails.
    fn validate_identity_nonces(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

pub(crate) trait StateTransitionHasIdentityNonceValidationV0 {
    /// True if the state transition has identity nonces validation.
    fn has_identity_nonce_validation(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error>;
}

impl StateTransitionIdentityNonceValidationV0 for StateTransition {
    fn validate_identity_nonces(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match self {
            StateTransition::Batch(st) => st.validate_identity_nonces(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            StateTransition::DataContractCreate(st) => st.validate_identity_nonces(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            StateTransition::DataContractUpdate(st) => st.validate_identity_nonces(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            StateTransition::IdentityUpdate(st) => st.validate_identity_nonces(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            StateTransition::IdentityCreditTransfer(st) => st.validate_identity_nonces(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            StateTransition::IdentityCreditWithdrawal(st) => st.validate_identity_nonces(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            StateTransition::MasternodeVote(st) => st.validate_identity_nonces(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            StateTransition::IdentityCreditTransferToAddresses(st) => st.validate_identity_nonces(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            StateTransition::AddressCreditWithdrawal(_)
            | StateTransition::AddressFundingFromAssetLock(_)
            | StateTransition::IdentityCreateFromAddresses(_)
            | StateTransition::IdentityTopUpFromAddresses(_)
            | StateTransition::AddressFundsTransfer(_)
            | StateTransition::IdentityCreate(_)
            | StateTransition::IdentityTopUp(_)
            | StateTransition::Shield(_)
            | StateTransition::ShieldedTransfer(_)
            | StateTransition::Unshield(_)
            | StateTransition::ShieldFromAssetLock(_)
            | StateTransition::ShieldedWithdrawal(_) => Ok(SimpleConsensusValidationResult::new()),
        }
    }
}

impl StateTransitionHasIdentityNonceValidationV0 for StateTransition {
    fn has_identity_nonce_validation(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .has_nonce_validation
        {
            0 => {
                let has_nonce_validation = matches!(
                    self,
                    StateTransition::Batch(_)
                        | StateTransition::DataContractCreate(_)
                        | StateTransition::DataContractUpdate(_)
                        | StateTransition::IdentityUpdate(_)
                        | StateTransition::IdentityCreditTransfer(_)
                        | StateTransition::IdentityCreditWithdrawal(_)
                );

                Ok(has_nonce_validation)
            }
            1 => {
                // Preferably to use match without wildcard arm (_) to avoid missing cases
                // in the future when new state transitions are added
                let has_nonce_validation = match self {
                    StateTransition::Batch(_)
                    | StateTransition::DataContractCreate(_)
                    | StateTransition::DataContractUpdate(_)
                    | StateTransition::IdentityUpdate(_)
                    | StateTransition::IdentityCreditTransfer(_)
                    | StateTransition::IdentityCreditWithdrawal(_)
                    | StateTransition::MasternodeVote(_)
                    | StateTransition::IdentityCreditTransferToAddresses(_) => true,
                    StateTransition::IdentityCreate(_)
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
                    | StateTransition::ShieldedWithdrawal(_) => false,
                };

                Ok(has_nonce_validation)
            }
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "StateTransition::has_nonce_validation".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
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

    mod has_identity_nonce_validation {
        use super::*;

        #[test]
        fn version_0_should_include_core_transitions_only() {
            // Version 0 includes: Batch, DataContractCreate, DataContractUpdate,
            // IdentityUpdate, IdentityCreditTransfer, IdentityCreditWithdrawal
            let platform_version = &platform_version::version::v1::PLATFORM_V1;

            let with_nonce: Vec<(&str, StateTransition)> = vec![
                (
                    "Batch",
                    StateTransition::Batch(BatchTransition::V0(BatchTransitionV0::default())),
                ),
                ("DataContractCreate", make_data_contract_create_st()),
                ("DataContractUpdate", make_data_contract_update_st()),
                (
                    "IdentityUpdate",
                    StateTransition::IdentityUpdate(IdentityUpdateTransition::V0(
                        IdentityUpdateTransitionV0::default(),
                    )),
                ),
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
            ];
            for (name, st) in with_nonce {
                assert!(
                    st.has_identity_nonce_validation(platform_version).unwrap(),
                    "expected has_identity_nonce_validation=true for {} on version 0",
                    name
                );
            }

            let without_nonce: Vec<(&str, StateTransition)> = vec![
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
            ];
            for (name, st) in without_nonce {
                assert!(
                    !st.has_identity_nonce_validation(platform_version).unwrap(),
                    "expected has_identity_nonce_validation=false for {} on version 0",
                    name
                );
            }
        }

        #[test]
        fn version_1_should_also_include_masternode_vote_and_credit_transfer_to_addresses() {
            // Version 1 (used from platform v8 onward) adds MasternodeVote and
            // IdentityCreditTransferToAddresses
            let platform_version = &platform_version::version::v8::PLATFORM_V8;

            let with_nonce: Vec<(&str, StateTransition)> = vec![
                (
                    "Batch",
                    StateTransition::Batch(BatchTransition::V0(BatchTransitionV0::default())),
                ),
                ("DataContractCreate", make_data_contract_create_st()),
                ("DataContractUpdate", make_data_contract_update_st()),
                (
                    "IdentityUpdate",
                    StateTransition::IdentityUpdate(IdentityUpdateTransition::V0(
                        IdentityUpdateTransitionV0::default(),
                    )),
                ),
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
                (
                    "MasternodeVote",
                    StateTransition::MasternodeVote(MasternodeVoteTransition::V0(
                        MasternodeVoteTransitionV0::default(),
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
            for (name, st) in with_nonce {
                assert!(
                    st.has_identity_nonce_validation(platform_version).unwrap(),
                    "expected has_identity_nonce_validation=true for {} on version 1",
                    name
                );
            }

            let without_nonce: Vec<(&str, StateTransition)> = vec![
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
                (
                    "AddressFundsTransfer",
                    StateTransition::AddressFundsTransfer(AddressFundsTransferTransition::V0(
                        AddressFundsTransferTransitionV0::default(),
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
                (
                    "AddressCreditWithdrawal",
                    StateTransition::AddressCreditWithdrawal(
                        AddressCreditWithdrawalTransition::V0(
                            AddressCreditWithdrawalTransitionV0::default(),
                        ),
                    ),
                ),
            ];
            for (name, st) in without_nonce {
                assert!(
                    !st.has_identity_nonce_validation(platform_version).unwrap(),
                    "expected has_identity_nonce_validation=false for {} on version 1",
                    name
                );
            }
        }

        #[test]
        fn should_return_error_for_unknown_version() {
            // Create a platform version with an invalid has_nonce_validation version
            let mut platform_version = platform_version::version::v1::PLATFORM_V1.clone();
            platform_version
                .drive_abci
                .validation_and_processing
                .has_nonce_validation = 99;
            let st = StateTransition::Batch(BatchTransition::V0(BatchTransitionV0::default()));
            let result = st.has_identity_nonce_validation(&platform_version);
            assert!(result.is_err());
        }
    }
}
