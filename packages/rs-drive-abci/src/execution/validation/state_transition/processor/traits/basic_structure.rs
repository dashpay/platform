use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::dashcore::Network;
use dpp::identity::state_transition::AssetLockProved;
use dpp::state_transition::{StateTransition, StateTransitionStructureValidation};
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

/// Validates the structure of the asset lock proof a funding transition mints credits from.
///
/// Every route that turns a Core asset lock into Platform credits owes this check. It binds the
/// instant lock to the transaction attached to the proof and validates that transaction's shape,
/// which is what makes the credited output actually backed by the locked coins. Signature
/// verification alone does not: a quorum signs the lock's own txid, not the attached transaction.
fn validate_asset_lock_proof_structure<T: AssetLockProved>(
    state_transition: &T,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    state_transition
        .asset_lock_proof()
        .validate_structure(platform_version)
        .map_err(Error::Protocol)
}

/// A trait for validating state transitions within a blockchain.
pub(crate) trait StateTransitionBasicStructureValidationV0 {
    /// Validates the structure of a transaction by checking its basic elements.
    ///
    /// # Arguments
    ///
    /// * `network_type` - The network we are on, mainnet/testnet/a devnet/a regtest.
    /// * `platform_version` - The platform version.
    ///
    /// # Returns
    ///
    /// * `Result<SimpleConsensusValidationResult, Error>` - A result with either a SimpleConsensusValidationResult or an Error.
    fn validate_basic_structure(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;

    /// True if the state transition has basic structure validation.
    /// Currently only data contract update does not
    fn has_basic_structure_validation(&self, _platform_version: &PlatformVersion) -> bool {
        true
    }
}

impl StateTransitionBasicStructureValidationV0 for StateTransition {
    fn validate_basic_structure(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match self {
            StateTransition::MasternodeVote(_) => {
                // no basic structure validation
                Ok(SimpleConsensusValidationResult::new())
            }
            StateTransition::IdentityCreate(st) => {
                st.validate_basic_structure(network_type, platform_version)
            }
            StateTransition::IdentityUpdate(st) => {
                st.validate_basic_structure(network_type, platform_version)
            }
            StateTransition::IdentityTopUp(st) => {
                st.validate_basic_structure(network_type, platform_version)
            }
            StateTransition::IdentityCreditWithdrawal(st) => {
                st.validate_basic_structure(network_type, platform_version)
            }
            StateTransition::Batch(st) => {
                st.validate_basic_structure(network_type, platform_version)
            }
            StateTransition::IdentityCreditTransfer(st) => {
                st.validate_basic_structure(network_type, platform_version)
            }
            StateTransition::DataContractCreate(st) => {
                if platform_version
                    .drive_abci
                    .validation_and_processing
                    .state_transitions
                    .contract_create_state_transition
                    .basic_structure
                    .is_some()
                {
                    st.validate_basic_structure(network_type, platform_version)
                } else {
                    Ok(SimpleConsensusValidationResult::new())
                }
            }
            StateTransition::DataContractUpdate(st) => {
                if platform_version
                    .drive_abci
                    .validation_and_processing
                    .state_transitions
                    .contract_update_state_transition
                    .basic_structure
                    .is_some()
                {
                    st.validate_basic_structure(network_type, platform_version)
                } else {
                    Ok(SimpleConsensusValidationResult::new())
                }
            }
            StateTransition::IdentityCreditTransferToAddresses(st) => {
                match platform_version
                    .drive_abci
                    .validation_and_processing
                    .state_transitions
                    .identity_credit_transfer_to_addresses_state_transition
                    .basic_structure
                {
                    Some(0) => {
                        // There is nothing expensive to add as validation methods to the execution context
                        Ok(st.validate_structure(platform_version))
                    }
                    Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                        method: "identity create from addresses transition: validate_basic_structure"
                            .to_string(),
                        known_versions: vec![0],
                        received: version,
                    })),
                    None => Err(Error::Execution(ExecutionError::VersionNotActive {
                        method: "identity create from addresses transition: validate_basic_structure"
                            .to_string(),
                        known_versions: vec![0],
                    })),
                }
            }
            StateTransition::IdentityCreateFromAddresses(st) => {
                match platform_version
                    .drive_abci
                    .validation_and_processing
                    .state_transitions
                    .identity_create_from_addresses_state_transition
                    .basic_structure
                {
                    Some(0) => {
                        // There is nothing expensive to add as validation methods to the execution context
                        Ok(st.validate_structure(platform_version))
                    }
                    Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                        method: "identity create from addresses transition: validate_basic_structure"
                            .to_string(),
                        known_versions: vec![0],
                        received: version,
                    })),
                    None => Err(Error::Execution(ExecutionError::VersionNotActive {
                        method: "identity create from addresses transition: validate_basic_structure"
                            .to_string(),
                        known_versions: vec![0],
                    })),
                }
            }
            StateTransition::IdentityTopUpFromAddresses(st) => {
                match platform_version
                    .drive_abci
                    .validation_and_processing
                    .state_transitions
                    .identity_top_up_from_addresses_state_transition
                    .basic_structure
                {
                    Some(0) => {
                        // There is nothing expensive to add as validation methods to the execution context
                        Ok(st.validate_structure(platform_version))
                    }
                    Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                        method: "identity create from addresses transition: validate_basic_structure"
                            .to_string(),
                        known_versions: vec![0],
                        received: version,
                    })),
                    None => Err(Error::Execution(ExecutionError::VersionNotActive {
                        method: "identity create from addresses transition: validate_basic_structure"
                            .to_string(),
                        known_versions: vec![0],
                    })),
                }
            }
            StateTransition::AddressFundsTransfer(st) => {
                match platform_version
                    .drive_abci
                    .validation_and_processing
                    .state_transitions
                    .address_funds_transfer
                    .basic_structure
                {
                    Some(0) => {
                        // There is nothing expensive to add as validation methods to the execution context
                        Ok(st.validate_structure(platform_version))
                    }
                    Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                        method: "identity create from addresses transition: validate_basic_structure"
                            .to_string(),
                        known_versions: vec![0],
                        received: version,
                    })),
                    None => Err(Error::Execution(ExecutionError::VersionNotActive {
                        method: "identity create from addresses transition: validate_basic_structure"
                            .to_string(),
                        known_versions: vec![0],
                    })),
                }
            }
            StateTransition::AddressFundingFromAssetLock(st) => {
                match platform_version
                    .drive_abci
                    .validation_and_processing
                    .state_transitions
                    .address_funds_from_asset_lock
                    .basic_structure
                {
                    Some(0) => {
                        // The asset lock proof must be validated before anything downstream reads
                        // the credited output: an instant lock only authenticates its own txid.
                        let result = validate_asset_lock_proof_structure(st, platform_version)?;
                        if !result.is_valid() {
                            return Ok(result);
                        }

                        // There is nothing expensive to add as validation methods to the execution context
                        Ok(st.validate_structure(platform_version))
                    }
                    Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                        method: "identity create from addresses transition: validate_basic_structure"
                            .to_string(),
                        known_versions: vec![0],
                        received: version,
                    })),
                    None => Err(Error::Execution(ExecutionError::VersionNotActive {
                        method: "identity create from addresses transition: validate_basic_structure"
                            .to_string(),
                        known_versions: vec![0],
                    })),
                }
            }
            StateTransition::AddressCreditWithdrawal(st) => {
                match platform_version
                    .drive_abci
                    .validation_and_processing
                    .state_transitions
                    .address_credit_withdrawal
                    .basic_structure
                {
                    Some(0) => {
                        // There is nothing expensive to add as validation methods to the execution context
                        Ok(st.validate_structure(platform_version))
                    }
                    Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                        method: "identity create from addresses transition: validate_basic_structure"
                            .to_string(),
                        known_versions: vec![0],
                        received: version,
                    })),
                    None => Err(Error::Execution(ExecutionError::VersionNotActive {
                        method: "identity create from addresses transition: validate_basic_structure"
                            .to_string(),
                        known_versions: vec![0],
                    })),
                }
            }
            StateTransition::Shield(st) => {
                match platform_version
                    .drive_abci
                    .validation_and_processing
                    .state_transitions
                    .shield_state_transition
                    .basic_structure
                {
                    Some(0) => Ok(st.validate_structure(platform_version)),
                    Some(version) => {
                        Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                            method: "shield transition: validate_basic_structure".to_string(),
                            known_versions: vec![0],
                            received: version,
                        }))
                    }
                    None => Err(Error::Execution(ExecutionError::VersionNotActive {
                        method: "shield transition: validate_basic_structure".to_string(),
                        known_versions: vec![0],
                    })),
                }
            }
            StateTransition::ShieldFromIdentity(st) => {
                match platform_version
                    .drive_abci
                    .validation_and_processing
                    .state_transitions
                    .shield_from_identity_state_transition
                    .basic_structure
                {
                    Some(0) => Ok(st.validate_structure(platform_version)),
                    Some(version) => {
                        Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                            method: "shield from identity transition: validate_basic_structure"
                                .to_string(),
                            known_versions: vec![0],
                            received: version,
                        }))
                    }
                    None => Err(Error::Execution(ExecutionError::VersionNotActive {
                        method: "shield from identity transition: validate_basic_structure"
                            .to_string(),
                        known_versions: vec![0],
                    })),
                }
            }
            StateTransition::ShieldedTransfer(st) => {
                match platform_version
                    .drive_abci
                    .validation_and_processing
                    .state_transitions
                    .shielded_transfer_state_transition
                    .basic_structure
                {
                    Some(0) => Ok(st.validate_structure(platform_version)),
                    Some(version) => {
                        Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                            method: "shielded transfer transition: validate_basic_structure"
                                .to_string(),
                            known_versions: vec![0],
                            received: version,
                        }))
                    }
                    None => Err(Error::Execution(ExecutionError::VersionNotActive {
                        method: "shielded transfer transition: validate_basic_structure"
                            .to_string(),
                        known_versions: vec![0],
                    })),
                }
            }
            StateTransition::Unshield(st) => {
                match platform_version
                    .drive_abci
                    .validation_and_processing
                    .state_transitions
                    .unshield_state_transition
                    .basic_structure
                {
                    Some(0) => Ok(st.validate_structure(platform_version)),
                    Some(version) => {
                        Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                            method: "unshield transition: validate_basic_structure".to_string(),
                            known_versions: vec![0],
                            received: version,
                        }))
                    }
                    None => Err(Error::Execution(ExecutionError::VersionNotActive {
                        method: "unshield transition: validate_basic_structure".to_string(),
                        known_versions: vec![0],
                    })),
                }
            }
            StateTransition::IdentityTopUpFromShieldedPool(st) => {
                match platform_version
                    .drive_abci
                    .validation_and_processing
                    .state_transitions
                    .identity_top_up_from_shielded_pool_state_transition
                    .basic_structure
                {
                    Some(0) => Ok(st.validate_structure(platform_version)),
                    Some(version) => {
                        Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                            method: "identity top up from shielded pool transition: validate_basic_structure".to_string(),
                            known_versions: vec![0],
                            received: version,
                        }))
                    }
                    None => Err(Error::Execution(ExecutionError::VersionNotActive {
                        method: "identity top up from shielded pool transition: validate_basic_structure".to_string(),
                        known_versions: vec![0],
                    })),
                }
            }
            StateTransition::ShieldFromAssetLock(st) => {
                match platform_version
                    .drive_abci
                    .validation_and_processing
                    .state_transitions
                    .shield_from_asset_lock_state_transition
                    .basic_structure
                {
                    Some(0) => {
                        // The asset lock proof must be validated before anything downstream reads
                        // the credited output: an instant lock only authenticates its own txid.
                        let result = validate_asset_lock_proof_structure(st, platform_version)?;
                        if !result.is_valid() {
                            return Ok(result);
                        }

                        Ok(st.validate_structure(platform_version))
                    }
                    Some(version) => {
                        Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                            method: "shield from asset lock transition: validate_basic_structure"
                                .to_string(),
                            known_versions: vec![0],
                            received: version,
                        }))
                    }
                    None => Err(Error::Execution(ExecutionError::VersionNotActive {
                        method: "shield from asset lock transition: validate_basic_structure"
                            .to_string(),
                        known_versions: vec![0],
                    })),
                }
            }
            StateTransition::ShieldedWithdrawal(st) => {
                match platform_version
                    .drive_abci
                    .validation_and_processing
                    .state_transitions
                    .shielded_withdrawal_state_transition
                    .basic_structure
                {
                    Some(0) => Ok(st.validate_structure(platform_version)),
                    Some(version) => {
                        Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                            method: "shielded withdrawal transition: validate_basic_structure"
                                .to_string(),
                            known_versions: vec![0],
                            received: version,
                        }))
                    }
                    None => Err(Error::Execution(ExecutionError::VersionNotActive {
                        method: "shielded withdrawal transition: validate_basic_structure"
                            .to_string(),
                        known_versions: vec![0],
                    })),
                }
            }
            StateTransition::IdentityCreateFromShieldedPool(st) => {
                match platform_version
                    .drive_abci
                    .validation_and_processing
                    .state_transitions
                    .identity_create_from_shielded_pool_state_transition
                    .basic_structure
                {
                    Some(0) => Ok(st.validate_structure(platform_version)),
                    Some(version) => {
                        Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                            method:
                                "identity create from shielded pool transition: validate_basic_structure"
                                    .to_string(),
                            known_versions: vec![0],
                            received: version,
                        }))
                    }
                    None => Err(Error::Execution(ExecutionError::VersionNotActive {
                        method:
                            "identity create from shielded pool transition: validate_basic_structure"
                                .to_string(),
                        known_versions: vec![0],
                    })),
                }
            }
        }
    }
    fn has_basic_structure_validation(&self, platform_version: &PlatformVersion) -> bool {
        match self {
            StateTransition::DataContractCreate(_) => {
                // Added in protocol version 9 (version 2.0)
                platform_version
                    .drive_abci
                    .validation_and_processing
                    .state_transitions
                    .contract_create_state_transition
                    .basic_structure
                    .is_some()
            }
            StateTransition::DataContractUpdate(_) => {
                // Added in protocol version 9  (version 2.0)
                platform_version
                    .drive_abci
                    .validation_and_processing
                    .state_transitions
                    .contract_update_state_transition
                    .basic_structure
                    .is_some()
            }
            StateTransition::Batch(_)
            | StateTransition::IdentityCreate(_)
            | StateTransition::IdentityTopUp(_)
            | StateTransition::IdentityCreditWithdrawal(_)
            | StateTransition::IdentityUpdate(_)
            | StateTransition::IdentityCreditTransfer(_)
            | StateTransition::AddressFundsTransfer(_)
            | StateTransition::IdentityCreditTransferToAddresses(_)
            | StateTransition::IdentityCreateFromAddresses(_)
            | StateTransition::IdentityTopUpFromAddresses(_)
            | StateTransition::AddressFundingFromAssetLock(_)
            | StateTransition::AddressCreditWithdrawal(_) => true,
            StateTransition::Shield(_) => platform_version
                .drive_abci
                .validation_and_processing
                .state_transitions
                .shield_state_transition
                .basic_structure
                .is_some(),
            StateTransition::ShieldFromIdentity(_) => platform_version
                .drive_abci
                .validation_and_processing
                .state_transitions
                .shield_from_identity_state_transition
                .basic_structure
                .is_some(),
            StateTransition::ShieldedTransfer(_) => platform_version
                .drive_abci
                .validation_and_processing
                .state_transitions
                .shielded_transfer_state_transition
                .basic_structure
                .is_some(),
            StateTransition::Unshield(_) => platform_version
                .drive_abci
                .validation_and_processing
                .state_transitions
                .unshield_state_transition
                .basic_structure
                .is_some(),
            StateTransition::IdentityTopUpFromShieldedPool(_) => platform_version
                .drive_abci
                .validation_and_processing
                .state_transitions
                .identity_top_up_from_shielded_pool_state_transition
                .basic_structure
                .is_some(),
            StateTransition::ShieldFromAssetLock(_) => platform_version
                .drive_abci
                .validation_and_processing
                .state_transitions
                .shield_from_asset_lock_state_transition
                .basic_structure
                .is_some(),
            StateTransition::ShieldedWithdrawal(_) => platform_version
                .drive_abci
                .validation_and_processing
                .state_transitions
                .shielded_withdrawal_state_transition
                .basic_structure
                .is_some(),
            StateTransition::IdentityCreateFromShieldedPool(_) => platform_version
                .drive_abci
                .validation_and_processing
                .state_transitions
                .identity_create_from_shielded_pool_state_transition
                .basic_structure
                .is_some(),
            StateTransition::MasternodeVote(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use dpp::state_transition::batch_transition::BatchTransition;
    use dpp::state_transition::batch_transition::BatchTransitionV0;
    use dpp::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
    use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
    use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
    use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
    use dpp::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;
    use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
    use dpp::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
    use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
    use dpp::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
    use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
    use dpp::state_transition::masternode_vote_transition::v0::MasternodeVoteTransitionV0;
    use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;

    mod asset_lock_proof_structure {
        use super::*;
        use dpp::address_funds::{AddressFundsFeeStrategyStep, PlatformAddress};
        use dpp::consensus::basic::BasicError;
        use dpp::consensus::ConsensusError;
        use dpp::dashcore::hashes::Hash;
        use dpp::dashcore::transaction::special_transaction::TransactionPayload;
        use dpp::dashcore::{OutPoint, ScriptBuf, Txid};
        use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
        use dpp::identity::state_transition::asset_lock_proof::AssetLockProof;
        use dpp::shielded::SerializedAction;
        use dpp::state_transition::address_funding_from_asset_lock_transition::v0::AddressFundingFromAssetLockTransitionV0;
        use dpp::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
        use dpp::state_transition::shield_from_asset_lock_transition::v0::ShieldFromAssetLockTransitionV0;
        use dpp::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
        use dpp::tests::fixtures::raw_instant_asset_lock_proof_fixture;
        use platform_version::version::feature_initial_protocol_versions::{
            ADDRESS_FUNDS_INITIAL_PROTOCOL_VERSION, SHIELDED_POOL_INITIAL_PROTOCOL_VERSION,
        };
        use platform_version::version::LATEST_VERSION;

        // Exercise the basic validation boundary shared by first-time CheckTx and consensus.
        // Signatures and Orchard proof bytes are placeholders: these tests never submit or
        // execute a transition and do not require Core or a signing quorum.
        fn funding_transitions(proof: AssetLockProof) -> Vec<(StateTransition, u32)> {
            vec![
                (
                    StateTransition::IdentityCreate(IdentityCreateTransition::V0(
                        IdentityCreateTransitionV0 {
                            asset_lock_proof: proof.clone(),
                            ..Default::default()
                        },
                    )),
                    1,
                ),
                (
                    StateTransition::IdentityTopUp(IdentityTopUpTransition::V0(
                        IdentityTopUpTransitionV0 {
                            asset_lock_proof: proof.clone(),
                            ..Default::default()
                        },
                    )),
                    1,
                ),
                (
                    StateTransition::AddressFundingFromAssetLock(
                        AddressFundingFromAssetLockTransition::V0(
                            AddressFundingFromAssetLockTransitionV0 {
                                asset_lock_proof: proof.clone(),
                                outputs: [(PlatformAddress::P2pkh([1; 20]), None)].into(),
                                fee_strategy: vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                                ..Default::default()
                            },
                        ),
                    ),
                    ADDRESS_FUNDS_INITIAL_PROTOCOL_VERSION,
                ),
                (
                    StateTransition::ShieldFromAssetLock(ShieldFromAssetLockTransition::V0(
                        ShieldFromAssetLockTransitionV0 {
                            asset_lock_proof: proof,
                            actions: vec![SerializedAction {
                                nullifier: [1; 32],
                                rk: [2; 32],
                                cmx: [3; 32],
                                encrypted_note: vec![4; 216],
                                cv_net: [5; 32],
                                spend_auth_sig: [0; 64],
                            }],
                            value_balance: 1_000_000,
                            anchor: [7; 32],
                            proof: vec![0; 100],
                            binding_signature: [0; 64],
                            surplus_output: None,
                            signature: Default::default(),
                        },
                    )),
                    SHIELDED_POOL_INITIAL_PROTOCOL_VERSION,
                ),
            ]
        }

        fn assert_proof_result(
            proof: AssetLockProof,
            check: impl Fn(&SimpleConsensusValidationResult) -> bool,
        ) {
            for (transition, initial_version) in funding_transitions(proof) {
                for protocol_version in initial_version..=LATEST_VERSION {
                    let platform_version = PlatformVersion::get(protocol_version).unwrap();
                    assert!(transition.has_basic_structure_validation(platform_version));
                    let result = transition
                        .validate_basic_structure(Network::Regtest, platform_version)
                        .expect("basic validation should return a consensus result");
                    assert!(
                        check(&result),
                        "unexpected result for {transition:?} at protocol {protocol_version}: {result:?}"
                    );
                }
            }
        }

        #[test]
        fn should_reject_mismatched_instant_lock_for_every_funding_transition() {
            let mut proof = raw_instant_asset_lock_proof_fixture(None, None);
            proof.instant_lock.txid = Txid::all_zeros();
            assert_proof_result(AssetLockProof::Instant(proof), |result| {
                matches!(
                    result.errors.as_slice(),
                    [ConsensusError::BasicError(
                        BasicError::IdentityAssetLockProofLockedTransactionMismatchError(_)
                    )]
                )
            });
        }

        #[test]
        fn should_accept_matching_instant_proof_structure_for_every_funding_transition() {
            let proof = raw_instant_asset_lock_proof_fixture(None, None);
            assert_proof_result(AssetLockProof::Instant(proof), |result| result.is_valid());
        }

        #[test]
        fn should_preserve_chain_proof_structure_validation_for_every_funding_transition() {
            let proof = AssetLockProof::Chain(ChainAssetLockProof {
                core_chain_locked_height: 100,
                out_point: OutPoint::new(Txid::all_zeros(), 0),
            });
            assert_proof_result(proof, |result| result.is_valid());
        }

        #[test]
        fn should_reject_missing_asset_lock_output_for_every_funding_transition() {
            let mut proof = raw_instant_asset_lock_proof_fixture(None, None);
            proof.output_index = 1;
            assert_proof_result(AssetLockProof::Instant(proof), |result| {
                matches!(
                    result.errors.as_slice(),
                    [ConsensusError::BasicError(
                        BasicError::IdentityAssetLockTransactionOutputNotFoundError(_)
                    )]
                )
            });
        }

        #[test]
        fn should_reject_missing_asset_lock_payload_for_every_funding_transition() {
            let mut proof = raw_instant_asset_lock_proof_fixture(None, None);
            proof.transaction.special_transaction_payload = None;
            proof.instant_lock.txid = proof.transaction.txid();
            assert_proof_result(AssetLockProof::Instant(proof), |result| {
                matches!(
                    result.errors.as_slice(),
                    [ConsensusError::BasicError(
                        BasicError::InvalidIdentityAssetLockTransactionError(_)
                    )]
                )
            });
        }

        #[test]
        fn should_reject_non_p2pkh_asset_lock_output_for_every_funding_transition() {
            let mut proof = raw_instant_asset_lock_proof_fixture(None, None);
            let Some(TransactionPayload::AssetLockPayloadType(payload)) =
                proof.transaction.special_transaction_payload.as_mut()
            else {
                panic!("fixture must contain an asset lock payload");
            };
            payload.credit_outputs[0].script_pubkey = ScriptBuf::new();
            proof.instant_lock.txid = proof.transaction.txid();
            assert_proof_result(AssetLockProof::Instant(proof), |result| {
                matches!(
                    result.errors.as_slice(),
                    [ConsensusError::BasicError(
                        BasicError::InvalidIdentityAssetLockTransactionOutputError(_)
                    )]
                )
            });
        }

        #[test]
        fn should_preserve_inactive_and_unknown_basic_structure_versions() {
            for (transition, initial_version) in
                funding_transitions(AssetLockProof::Chain(ChainAssetLockProof {
                    core_chain_locked_height: 100,
                    out_point: OutPoint::new(Txid::all_zeros(), 0),
                }))
            {
                if initial_version == 1 {
                    continue;
                }
                for version in [None, Some(u16::MAX)] {
                    let mut platform_version = PlatformVersion::latest().clone();
                    let versions = &mut platform_version
                        .drive_abci
                        .validation_and_processing
                        .state_transitions;
                    match &transition {
                        StateTransition::AddressFundingFromAssetLock(_) => {
                            versions.address_funds_from_asset_lock.basic_structure = version;
                        }
                        StateTransition::ShieldFromAssetLock(_) => {
                            versions
                                .shield_from_asset_lock_state_transition
                                .basic_structure = version;
                        }
                        _ => unreachable!(),
                    }
                    let result =
                        transition.validate_basic_structure(Network::Regtest, &platform_version);
                    match version {
                        None => assert!(matches!(
                            result,
                            Err(Error::Execution(ExecutionError::VersionNotActive { .. }))
                        )),
                        Some(_) => assert!(matches!(
                            result,
                            Err(Error::Execution(
                                ExecutionError::UnknownVersionMismatch { .. }
                            ))
                        )),
                    }
                }
            }
        }
    }

    mod has_basic_structure_validation {
        use super::*;

        #[test]
        fn should_return_true_for_always_validated_transitions() {
            let platform_version = &platform_version::version::v1::PLATFORM_V1;
            let transitions: Vec<(&str, StateTransition)> = vec![
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
                    "IdentityTopUp",
                    StateTransition::IdentityTopUp(IdentityTopUpTransition::V0(
                        IdentityTopUpTransitionV0::default(),
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
            ];
            for (name, st) in transitions {
                assert!(
                    st.has_basic_structure_validation(platform_version),
                    "expected true for {}",
                    name
                );
            }
        }

        #[test]
        fn should_return_false_for_masternode_vote() {
            let platform_version = &platform_version::version::v1::PLATFORM_V1;
            let st = StateTransition::MasternodeVote(MasternodeVoteTransition::V0(
                MasternodeVoteTransitionV0::default(),
            ));
            assert!(!st.has_basic_structure_validation(platform_version));
        }
    }

    /// Every route that mints Platform credits from a Core asset lock must reject a proof whose
    /// instant lock authenticates a different transaction than the one attached to it.
    ///
    /// A quorum signs the lock's own txid and nothing else, so a lock paired with an unrelated
    /// transaction is a valid signature over an object that says nothing about the output being
    /// credited. Skipping this check on any one route lets an attacker mint unbacked credits.
    mod asset_lock_transaction_binding {
        use super::*;

        use dpp::address_funds::PlatformAddress;
        use dpp::consensus::basic::BasicError;
        use dpp::consensus::ConsensusError;
        use dpp::identity::state_transition::asset_lock_proof::{
            AssetLockProof, InstantAssetLockProof,
        };
        use dpp::state_transition::address_funding_from_asset_lock_transition::v0::AddressFundingFromAssetLockTransitionV0;
        use dpp::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
        use dpp::state_transition::shield_from_asset_lock_transition::v0::ShieldFromAssetLockTransitionV0;
        use dpp::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
        use dpp::tests::fixtures::instant_asset_lock_proof_fixture;
        use std::collections::BTreeMap;

        /// Says whether a transition funds Platform from a Core asset lock.
        ///
        /// The match is deliberately exhaustive: adding a state transition variant stops this
        /// function compiling, which forces whoever adds it to answer the question. Everything
        /// answering `true` needs a case in [`transitions_funded_by_an_asset_lock`].
        fn is_funded_by_an_asset_lock(state_transition: &StateTransition) -> bool {
            match state_transition {
                StateTransition::IdentityCreate(_)
                | StateTransition::IdentityTopUp(_)
                | StateTransition::AddressFundingFromAssetLock(_)
                | StateTransition::ShieldFromAssetLock(_) => true,
                StateTransition::DataContractCreate(_)
                | StateTransition::DataContractUpdate(_)
                | StateTransition::Batch(_)
                | StateTransition::IdentityCreditWithdrawal(_)
                | StateTransition::IdentityUpdate(_)
                | StateTransition::IdentityCreditTransfer(_)
                | StateTransition::MasternodeVote(_)
                | StateTransition::ShieldFromIdentity(_)
                | StateTransition::IdentityCreditTransferToAddresses(_)
                | StateTransition::IdentityCreateFromAddresses(_)
                | StateTransition::IdentityTopUpFromAddresses(_)
                | StateTransition::AddressFundsTransfer(_)
                | StateTransition::AddressCreditWithdrawal(_)
                | StateTransition::Shield(_)
                | StateTransition::ShieldedTransfer(_)
                | StateTransition::IdentityTopUpFromShieldedPool(_)
                | StateTransition::Unshield(_)
                | StateTransition::ShieldedWithdrawal(_)
                | StateTransition::IdentityCreateFromShieldedPool(_) => false,
            }
        }

        /// An instant proof whose lock commits to one transaction while a different one is attached.
        fn mismatched_instant_asset_lock_proof() -> AssetLockProof {
            let AssetLockProof::Instant(locked) = instant_asset_lock_proof_fixture(None, None)
            else {
                panic!("the fixture should produce an instant asset lock proof");
            };
            let AssetLockProof::Instant(attached) = instant_asset_lock_proof_fixture(None, None)
            else {
                panic!("the fixture should produce an instant asset lock proof");
            };

            assert_ne!(
                locked.instant_lock().txid,
                attached.transaction().txid(),
                "the two fixture transactions must differ for this to be a mismatch"
            );

            AssetLockProof::Instant(InstantAssetLockProof::new(
                locked.instant_lock().clone(),
                attached.transaction().clone(),
                attached.output_index(),
            ))
        }

        /// One transition per asset-lock-funded variant, each carrying a mismatched proof.
        fn transitions_funded_by_an_asset_lock() -> Vec<(&'static str, StateTransition)> {
            vec![
                (
                    "IdentityCreate",
                    StateTransition::IdentityCreate(IdentityCreateTransition::V0(
                        IdentityCreateTransitionV0 {
                            asset_lock_proof: mismatched_instant_asset_lock_proof(),
                            ..Default::default()
                        },
                    )),
                ),
                (
                    "IdentityTopUp",
                    StateTransition::IdentityTopUp(IdentityTopUpTransition::V0(
                        IdentityTopUpTransitionV0 {
                            asset_lock_proof: mismatched_instant_asset_lock_proof(),
                            ..Default::default()
                        },
                    )),
                ),
                (
                    "AddressFundingFromAssetLock",
                    StateTransition::AddressFundingFromAssetLock(
                        AddressFundingFromAssetLockTransition::V0(
                            AddressFundingFromAssetLockTransitionV0 {
                                asset_lock_proof: mismatched_instant_asset_lock_proof(),
                                outputs: BTreeMap::from([(
                                    PlatformAddress::P2pkh([1u8; 20]),
                                    None,
                                )]),
                                ..Default::default()
                            },
                        ),
                    ),
                ),
                (
                    "ShieldFromAssetLock",
                    StateTransition::ShieldFromAssetLock(ShieldFromAssetLockTransition::V0(
                        ShieldFromAssetLockTransitionV0 {
                            asset_lock_proof: mismatched_instant_asset_lock_proof(),
                            actions: vec![],
                            value_balance: 1_000,
                            anchor: [7u8; 32],
                            proof: vec![8u8; 100],
                            binding_signature: [9u8; 64],
                            surplus_output: None,
                            signature: Default::default(),
                        },
                    )),
                ),
            ]
        }

        #[test]
        fn every_asset_lock_funded_transition_rejects_a_locked_transaction_mismatch() {
            let platform_version = PlatformVersion::latest();

            for (name, state_transition) in transitions_funded_by_an_asset_lock() {
                assert!(
                    is_funded_by_an_asset_lock(&state_transition),
                    "{} is listed as asset lock funded but the roster disagrees",
                    name
                );

                let result = state_transition
                    .validate_basic_structure(Network::Testnet, platform_version)
                    .unwrap_or_else(|e| panic!("{} should validate basic structure: {}", name, e));

                assert_matches::assert_matches!(
                    result.errors.as_slice(),
                    [ConsensusError::BasicError(
                        BasicError::IdentityAssetLockProofLockedTransactionMismatchError(_)
                    )],
                    "{} accepted an instant lock that authenticates a different transaction",
                    name
                );
            }
        }
    }
}
