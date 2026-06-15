use dpp::identity::PartialIdentity;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::StateTransition;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use dpp::version::PlatformVersion;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::common::validate_state_transition_identity_signed::ValidateStateTransitionIdentitySignature;
use crate::execution::validation::state_transition::identity_credit_withdrawal::signature_purpose_matches_requirements::IdentityCreditWithdrawalStateTransitionSignaturePurposeMatchesRequirementsValidation;
use crate::execution::validation::state_transition::identity_top_up::identity_retrieval::v0::IdentityTopUpStateTransitionIdentityRetrievalV0;

/// A trait for validating state transitions within a blockchain.
pub(crate) trait StateTransitionIdentityBasedSignatureValidationV0 {
    /// Validates the identity and signatures of a transaction to ensure its authenticity.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the drive containing the transaction data.
    /// * `tx` - The transaction argument to be authenticated.
    /// * `execution_context` - A mutable reference to the StateTransitionExecutionContext that provides the context for validation.
    /// * `platform_version` - A reference to the PlatformVersion to be used for validation.
    ///
    /// # Returns
    ///
    /// Returns a `Result` with either:
    /// - `Ok(ConsensusValidationResult<Option<PartialIdentity>>)`: Indicates that the transaction has passed authentication, and the result contains an optional `PartialIdentity`.
    /// - `Err(Error)`: Indicates that the transaction failed authentication, and the result contains an `Error` indicating the reason for failure.
    ///
    fn validate_identity_signed_state_transition(
        &self,
        drive: &Drive,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<PartialIdentity>, Error>;

    /// fetches identity info
    fn retrieve_identity_info(
        &self,
        drive: &Drive,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<PartialIdentity>, Error>;

    /// Is the state transition supposed to have an identity in the state to succeed
    fn uses_identity_in_state(&self) -> bool;

    /// Do we validate the signature based on identity info?
    fn validates_signature_based_on_identity_info(&self) -> bool;
}

impl StateTransitionIdentityBasedSignatureValidationV0 for StateTransition {
    fn validate_identity_signed_state_transition(
        &self,
        drive: &Drive,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<PartialIdentity>, Error> {
        match self {
            StateTransition::DataContractCreate(_)
            | StateTransition::DataContractUpdate(_)
            | StateTransition::IdentityCreditTransfer(_)
            | StateTransition::Batch(_)
            | StateTransition::IdentityCreditTransferToAddresses(_) => {
                //Basic signature verification
                Ok(self.validate_state_transition_identity_signed(
                    drive,
                    true,
                    false,
                    tx,
                    execution_context,
                    platform_version,
                )?)
            }
            StateTransition::IdentityCreditWithdrawal(credit_withdrawal) => {
                let mut consensus_validation_result = self
                    .validate_state_transition_identity_signed(
                        drive,
                        true,
                        false,
                        tx,
                        execution_context,
                        platform_version,
                    )?;

                if consensus_validation_result.is_valid_with_data() {
                    let validation_result = credit_withdrawal
                        .validate_signature_purpose_matches_requirements(
                            consensus_validation_result.data.as_ref().unwrap(),
                            platform_version,
                        )?;
                    if !validation_result.is_valid() {
                        consensus_validation_result.add_errors(validation_result.errors);
                    }
                }
                Ok(consensus_validation_result)
            }
            StateTransition::IdentityUpdate(_) => {
                //Basic signature verification
                Ok(self.validate_state_transition_identity_signed(
                    drive,
                    true,
                    true,
                    tx,
                    execution_context,
                    platform_version,
                )?)
            }
            StateTransition::MasternodeVote(_) => {
                //Basic signature verification

                // We do not request the balance because masternodes do not pay for their voting
                //  themselves

                Ok(self.validate_state_transition_identity_signed(
                    drive,
                    false,
                    false,
                    tx,
                    execution_context,
                    platform_version,
                )?)
            }
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
            | StateTransition::ShieldedWithdrawal(_)
            | StateTransition::IdentityCreateFromShieldedPool(_) => {
                Ok(ConsensusValidationResult::new())
            }
        }
    }

    fn retrieve_identity_info(
        &self,
        drive: &Drive,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<PartialIdentity>, Error> {
        match self {
            StateTransition::IdentityTopUp(st) => Ok(st.retrieve_topped_up_identity(
                drive,
                tx,
                execution_context,
                platform_version,
            )?),
            StateTransition::IdentityTopUpFromAddresses(st) => Ok(st.retrieve_topped_up_identity(
                drive,
                tx,
                execution_context,
                platform_version,
            )?),
            _ => Ok(ConsensusValidationResult::new()),
        }
    }

    /// Is the state transition supposed to have an identity in the state to succeed
    fn uses_identity_in_state(&self) -> bool {
        match self {
            StateTransition::IdentityCreate(_)
            | StateTransition::IdentityCreateFromAddresses(_)
            | StateTransition::AddressFundsTransfer(_)
            | StateTransition::AddressFundingFromAssetLock(_)
            | StateTransition::AddressCreditWithdrawal(_)
            | StateTransition::Shield(_)
            | StateTransition::ShieldedTransfer(_)
            | StateTransition::Unshield(_)
            | StateTransition::ShieldFromAssetLock(_)
            | StateTransition::ShieldedWithdrawal(_)
            | StateTransition::IdentityCreateFromShieldedPool(_) => false,
            StateTransition::DataContractCreate(_)
            | StateTransition::DataContractUpdate(_)
            | StateTransition::Batch(_)
            | StateTransition::IdentityTopUp(_)
            | StateTransition::IdentityCreditWithdrawal(_)
            | StateTransition::IdentityUpdate(_)
            | StateTransition::IdentityCreditTransfer(_)
            | StateTransition::MasternodeVote(_)
            | StateTransition::IdentityCreditTransferToAddresses(_)
            | StateTransition::IdentityTopUpFromAddresses(_) => true,
        }
    }

    /// Do we validate the signature based on identity info?
    fn validates_signature_based_on_identity_info(&self) -> bool {
        match self {
            StateTransition::IdentityCreate(_)
            | StateTransition::IdentityCreateFromAddresses(_)
            | StateTransition::AddressFundsTransfer(_)
            | StateTransition::AddressFundingFromAssetLock(_)
            | StateTransition::AddressCreditWithdrawal(_)
            | StateTransition::IdentityTopUpFromAddresses(_)
            | StateTransition::IdentityTopUp(_)
            | StateTransition::Shield(_)
            | StateTransition::ShieldedTransfer(_)
            | StateTransition::Unshield(_)
            | StateTransition::ShieldFromAssetLock(_)
            | StateTransition::ShieldedWithdrawal(_)
            | StateTransition::IdentityCreateFromShieldedPool(_) => false,
            StateTransition::DataContractCreate(_)
            | StateTransition::DataContractUpdate(_)
            | StateTransition::Batch(_)
            | StateTransition::IdentityCreditWithdrawal(_)
            | StateTransition::IdentityUpdate(_)
            | StateTransition::IdentityCreditTransfer(_)
            | StateTransition::MasternodeVote(_)
            | StateTransition::IdentityCreditTransferToAddresses(_) => true,
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
    use dpp::state_transition::shield_transition::ShieldTransition;
    use dpp::state_transition::shield_transition::v0::ShieldTransitionV0;
    use dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
    use dpp::state_transition::shielded_transfer_transition::v0::ShieldedTransferTransitionV0;
    use dpp::state_transition::unshield_transition::UnshieldTransition;
    use dpp::state_transition::unshield_transition::v0::UnshieldTransitionV0;
    use dpp::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
    use dpp::state_transition::shield_from_asset_lock_transition::v0::ShieldFromAssetLockTransitionV0;
    use dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
    use dpp::state_transition::shielded_withdrawal_transition::v0::ShieldedWithdrawalTransitionV0;

    fn make_shield() -> StateTransition {
        StateTransition::Shield(ShieldTransition::V0(ShieldTransitionV0 {
            inputs: Default::default(),
            actions: vec![],
            amount: 0,
            anchor: [0u8; 32],
            proof: vec![],
            binding_signature: [0u8; 64],
            fee_strategy: vec![],
            user_fee_increase: 0,
            input_witnesses: vec![],
        }))
    }
    fn make_shielded_transfer() -> StateTransition {
        StateTransition::ShieldedTransfer(ShieldedTransferTransition::V0(
            ShieldedTransferTransitionV0 {
                actions: vec![],
                value_balance: 0,
                anchor: [0u8; 32],
                proof: vec![],
                binding_signature: [0u8; 64],
            },
        ))
    }
    fn make_unshield() -> StateTransition {
        StateTransition::Unshield(UnshieldTransition::V0(UnshieldTransitionV0 {
            output_address: Default::default(),
            actions: vec![],
            unshielding_amount: 0,
            anchor: [0u8; 32],
            proof: vec![],
            binding_signature: [0u8; 64],
        }))
    }
    fn make_shield_from_asset_lock() -> StateTransition {
        StateTransition::ShieldFromAssetLock(ShieldFromAssetLockTransition::V0(
            ShieldFromAssetLockTransitionV0 {
                asset_lock_proof: Default::default(),
                actions: vec![],
                value_balance: 0,
                anchor: [0u8; 32],
                proof: vec![],
                binding_signature: [0u8; 64],
                surplus_output: None,
                signature: Default::default(),
            },
        ))
    }
    fn make_shielded_withdrawal() -> StateTransition {
        StateTransition::ShieldedWithdrawal(ShieldedWithdrawalTransition::V0(
            ShieldedWithdrawalTransitionV0 {
                actions: vec![],
                unshielding_amount: 0,
                anchor: [0u8; 32],
                proof: vec![],
                binding_signature: [0u8; 64],
                core_fee_per_byte: 0,
                pooling: Default::default(),
                output_script: Default::default(),
            },
        ))
    }

    /// Transitions that do NOT use an identity in state (identity create, address-based, shielded).
    fn transitions_not_using_identity_in_state() -> Vec<(&'static str, StateTransition)> {
        vec![
            (
                "IdentityCreate",
                StateTransition::IdentityCreate(IdentityCreateTransition::V0(
                    IdentityCreateTransitionV0::default(),
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
                StateTransition::AddressCreditWithdrawal(AddressCreditWithdrawalTransition::V0(
                    AddressCreditWithdrawalTransitionV0::default(),
                )),
            ),
            ("Shield", make_shield()),
            ("ShieldedTransfer", make_shielded_transfer()),
            ("Unshield", make_unshield()),
            ("ShieldFromAssetLock", make_shield_from_asset_lock()),
            ("ShieldedWithdrawal", make_shielded_withdrawal()),
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
        ]
    }

    /// Transitions that DO use an identity in state.
    fn transitions_using_identity_in_state() -> Vec<(&'static str, StateTransition)> {
        vec![
            ("DataContractCreate", make_data_contract_create_st()),
            ("DataContractUpdate", make_data_contract_update_st()),
            (
                "Batch",
                StateTransition::Batch(BatchTransition::V0(BatchTransitionV0::default())),
            ),
            (
                "IdentityTopUp",
                StateTransition::IdentityTopUp(IdentityTopUpTransition::V0(
                    IdentityTopUpTransitionV0::default(),
                )),
            ),
            (
                "IdentityCreditWithdrawal",
                StateTransition::IdentityCreditWithdrawal(IdentityCreditWithdrawalTransition::V0(
                    IdentityCreditWithdrawalTransitionV0::default(),
                )),
            ),
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
            (
                "IdentityTopUpFromAddresses",
                StateTransition::IdentityTopUpFromAddresses(
                    IdentityTopUpFromAddressesTransition::V0(
                        IdentityTopUpFromAddressesTransitionV0::default(),
                    ),
                ),
            ),
        ]
    }

    mod uses_identity_in_state {
        use super::*;

        #[test]
        fn should_return_false_for_identity_create_and_address_based_transitions() {
            for (name, st) in transitions_not_using_identity_in_state() {
                assert!(!st.uses_identity_in_state(), "expected false for {}", name);
            }
        }

        #[test]
        fn should_return_true_for_identity_based_transitions() {
            for (name, st) in transitions_using_identity_in_state() {
                assert!(st.uses_identity_in_state(), "expected true for {}", name);
            }
        }
    }

    mod validates_signature_based_on_identity_info {
        use super::*;

        #[test]
        fn should_return_true_for_identity_signed_transitions() {
            let transitions_with_sig_validation: Vec<(&str, StateTransition)> = vec![
                ("DataContractCreate", make_data_contract_create_st()),
                ("DataContractUpdate", make_data_contract_update_st()),
                (
                    "Batch",
                    StateTransition::Batch(BatchTransition::V0(BatchTransitionV0::default())),
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
            for (name, st) in transitions_with_sig_validation {
                assert!(
                    st.validates_signature_based_on_identity_info(),
                    "expected true for {}",
                    name
                );
            }
        }

        #[test]
        fn should_return_false_for_non_identity_signed_transitions() {
            let transitions_without_sig_validation: Vec<(&str, StateTransition)> = vec![
                (
                    "IdentityCreate",
                    StateTransition::IdentityCreate(IdentityCreateTransition::V0(
                        IdentityCreateTransitionV0::default(),
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
                (
                    "IdentityTopUpFromAddresses",
                    StateTransition::IdentityTopUpFromAddresses(
                        IdentityTopUpFromAddressesTransition::V0(
                            IdentityTopUpFromAddressesTransitionV0::default(),
                        ),
                    ),
                ),
                (
                    "IdentityTopUp",
                    StateTransition::IdentityTopUp(IdentityTopUpTransition::V0(
                        IdentityTopUpTransitionV0::default(),
                    )),
                ),
                ("Shield", make_shield()),
                ("ShieldedTransfer", make_shielded_transfer()),
                ("Unshield", make_unshield()),
                ("ShieldFromAssetLock", make_shield_from_asset_lock()),
                ("ShieldedWithdrawal", make_shielded_withdrawal()),
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
            for (name, st) in transitions_without_sig_validation {
                assert!(
                    !st.validates_signature_based_on_identity_info(),
                    "expected false for {}",
                    name
                );
            }
        }
    }
}
