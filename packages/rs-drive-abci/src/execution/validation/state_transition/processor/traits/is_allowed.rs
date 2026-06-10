use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::consensus::basic::state_transition::StateTransitionNotActiveError;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::StateTransition;
use dpp::version::feature_initial_protocol_versions::{
    ADDRESS_FUNDS_INITIAL_PROTOCOL_VERSION, SHIELDED_POOL_INITIAL_PROTOCOL_VERSION,
};
use dpp::version::PlatformVersion;

/// A trait for validating state transitions within a blockchain.
pub(crate) trait StateTransitionIsAllowedValidationV0 {
    /// This means we should validate is state transition is allowed
    fn has_is_allowed_validation(&self) -> Result<bool, Error>;
    /// Preliminary validation for a state transition
    fn validate_is_allowed<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<()>, Error>;
}

impl StateTransitionIsAllowedValidationV0 for StateTransition {
    fn has_is_allowed_validation(&self) -> Result<bool, Error> {
        match self {
            StateTransition::Batch(_)
            | StateTransition::IdentityTopUpFromAddresses(_)
            | StateTransition::IdentityCreateFromAddresses(_)
            | StateTransition::AddressFundsTransfer(_)
            | StateTransition::IdentityCreditTransferToAddresses(_)
            | StateTransition::AddressFundingFromAssetLock(_)
            | StateTransition::AddressCreditWithdrawal(_)
            | StateTransition::Shield(_)
            | StateTransition::ShieldedTransfer(_)
            | StateTransition::Unshield(_)
            | StateTransition::ShieldFromAssetLock(_)
            | StateTransition::ShieldedWithdrawal(_)
            | StateTransition::IdentityCreateFromShieldedPool(_) => Ok(true),
            StateTransition::DataContractCreate(_)
            | StateTransition::DataContractUpdate(_)
            | StateTransition::IdentityCreate(_)
            | StateTransition::IdentityTopUp(_)
            | StateTransition::IdentityCreditWithdrawal(_)
            | StateTransition::IdentityUpdate(_)
            | StateTransition::IdentityCreditTransfer(_)
            | StateTransition::MasternodeVote(_) => Ok(false),
        }
    }

    fn validate_is_allowed<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<()>, Error> {
        match self {
            StateTransition::Batch(st) => st.validate_is_allowed(platform, platform_version),
            StateTransition::IdentityTopUpFromAddresses(_)
            | StateTransition::IdentityCreateFromAddresses(_)
            | StateTransition::AddressFundsTransfer(_)
            | StateTransition::IdentityCreditTransferToAddresses(_)
            | StateTransition::AddressFundingFromAssetLock(_)
            | StateTransition::AddressCreditWithdrawal(_) => {
                if platform_version.protocol_version >= ADDRESS_FUNDS_INITIAL_PROTOCOL_VERSION {
                    Ok(ConsensusValidationResult::new())
                } else {
                    Ok(ConsensusValidationResult::new_with_errors(vec![
                        StateTransitionNotActiveError::new(
                            self.state_transition_type().to_string(),
                            platform_version.protocol_version,
                            ADDRESS_FUNDS_INITIAL_PROTOCOL_VERSION,
                        )
                        .into(),
                    ]))
                }
            }
            StateTransition::Shield(_)
            | StateTransition::ShieldedTransfer(_)
            | StateTransition::Unshield(_)
            | StateTransition::ShieldFromAssetLock(_)
            | StateTransition::ShieldedWithdrawal(_)
            | StateTransition::IdentityCreateFromShieldedPool(_) => {
                if platform_version.protocol_version >= SHIELDED_POOL_INITIAL_PROTOCOL_VERSION {
                    Ok(ConsensusValidationResult::new())
                } else {
                    Ok(ConsensusValidationResult::new_with_errors(vec![
                        StateTransitionNotActiveError::new(
                            self.state_transition_type().to_string(),
                            platform_version.protocol_version,
                            SHIELDED_POOL_INITIAL_PROTOCOL_VERSION,
                        )
                        .into(),
                    ]))
                }
            }
            _ => Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "validate_is_allowed is not implemented for this state transition",
            ))),
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

    fn make_shield_transition() -> StateTransition {
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

    fn make_shielded_transfer_transition() -> StateTransition {
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

    fn make_unshield_transition() -> StateTransition {
        StateTransition::Unshield(UnshieldTransition::V0(UnshieldTransitionV0 {
            output_address: Default::default(),
            actions: vec![],
            unshielding_amount: 0,
            anchor: [0u8; 32],
            proof: vec![],
            binding_signature: [0u8; 64],
        }))
    }

    fn make_shield_from_asset_lock_transition() -> StateTransition {
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

    fn make_shielded_withdrawal_transition() -> StateTransition {
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

    fn make_identity_create_from_shielded_pool_transition() -> StateTransition {
        use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::v0::IdentityCreateFromShieldedPoolTransitionV0;
        use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition;
        StateTransition::IdentityCreateFromShieldedPool(
            IdentityCreateFromShieldedPoolTransition::V0(
                IdentityCreateFromShieldedPoolTransitionV0 {
                    public_keys: vec![],
                    denomination: 0,
                    actions: vec![],
                    anchor: [0u8; 32],
                    proof: vec![],
                    binding_signature: [0u8; 64],
                    send_to_address_on_creation_failure: dpp::address_funds::PlatformAddress::P2pkh(
                        [0u8; 20],
                    ),
                    identity_id: Default::default(),
                },
            ),
        )
    }

    /// Returns all state transitions grouped by expected `has_is_allowed_validation` result.
    fn transitions_requiring_allowed_validation() -> Vec<StateTransition> {
        vec![
            StateTransition::Batch(BatchTransition::V0(BatchTransitionV0::default())),
            StateTransition::IdentityTopUpFromAddresses(IdentityTopUpFromAddressesTransition::V0(
                IdentityTopUpFromAddressesTransitionV0::default(),
            )),
            StateTransition::IdentityCreateFromAddresses(
                IdentityCreateFromAddressesTransition::V0(
                    IdentityCreateFromAddressesTransitionV0::default(),
                ),
            ),
            StateTransition::AddressFundsTransfer(AddressFundsTransferTransition::V0(
                AddressFundsTransferTransitionV0::default(),
            )),
            StateTransition::IdentityCreditTransferToAddresses(
                IdentityCreditTransferToAddressesTransition::V0(
                    IdentityCreditTransferToAddressesTransitionV0::default(),
                ),
            ),
            StateTransition::AddressFundingFromAssetLock(
                AddressFundingFromAssetLockTransition::V0(
                    AddressFundingFromAssetLockTransitionV0::default(),
                ),
            ),
            StateTransition::AddressCreditWithdrawal(AddressCreditWithdrawalTransition::V0(
                AddressCreditWithdrawalTransitionV0::default(),
            )),
            make_shield_transition(),
            make_shielded_transfer_transition(),
            make_unshield_transition(),
            make_shield_from_asset_lock_transition(),
            make_shielded_withdrawal_transition(),
            make_identity_create_from_shielded_pool_transition(),
        ]
    }

    fn transitions_not_requiring_allowed_validation() -> Vec<StateTransition> {
        vec![
            make_data_contract_create_st(),
            make_data_contract_update_st(),
            StateTransition::IdentityCreate(IdentityCreateTransition::V0(
                IdentityCreateTransitionV0::default(),
            )),
            StateTransition::IdentityTopUp(IdentityTopUpTransition::V0(
                IdentityTopUpTransitionV0::default(),
            )),
            StateTransition::IdentityCreditWithdrawal(IdentityCreditWithdrawalTransition::V0(
                IdentityCreditWithdrawalTransitionV0::default(),
            )),
            StateTransition::IdentityUpdate(IdentityUpdateTransition::V0(
                IdentityUpdateTransitionV0::default(),
            )),
            StateTransition::IdentityCreditTransfer(IdentityCreditTransferTransition::V0(
                IdentityCreditTransferTransitionV0::default(),
            )),
            StateTransition::MasternodeVote(MasternodeVoteTransition::V0(
                MasternodeVoteTransitionV0::default(),
            )),
        ]
    }

    mod has_is_allowed_validation {
        use super::*;

        #[test]
        fn should_return_true_for_transitions_requiring_allowed_check() {
            for st in transitions_requiring_allowed_validation() {
                assert!(
                    st.has_is_allowed_validation().unwrap(),
                    "expected has_is_allowed_validation=true for {:?}",
                    std::mem::discriminant(&st)
                );
            }
        }

        #[test]
        fn should_return_false_for_transitions_not_requiring_allowed_check() {
            for st in transitions_not_requiring_allowed_validation() {
                assert!(
                    !st.has_is_allowed_validation().unwrap(),
                    "expected has_is_allowed_validation=false for {:?}",
                    std::mem::discriminant(&st)
                );
            }
        }
    }
}
