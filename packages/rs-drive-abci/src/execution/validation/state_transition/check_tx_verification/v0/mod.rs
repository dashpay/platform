use crate::error::Error;
use crate::execution::types::execution_event::ExecutionEvent;
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformer;
use crate::platform_types::platform::PlatformRef;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::identity::state_transition::OptionallyAssetLockProved;
use dpp::prelude::ConsensusValidationResult;
use dpp::serialization::Signable;
use dpp::state_transition::signable_bytes_hasher::SignableBytesHasher;
use dpp::ProtocolError;
use dpp::state_transition::StateTransition;
use dpp::version::{DefaultForPlatformVersion, PlatformVersion};
use crate::error::execution::ExecutionError;
use crate::execution::check_tx::CheckTxLevel;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::common::asset_lock::proof::verify_is_not_spent::AssetLockProofVerifyIsNotSpent;
use crate::execution::validation::state_transition::processor::address_witnesses::{StateTransitionAddressWitnessValidationV0, StateTransitionHasAddressWitnessValidationV0};
use crate::execution::validation::state_transition::processor::traits::shielded_proof::{StateTransitionHasShieldedProofValidationV0, StateTransitionShieldedMinimumFeeValidationV0, StateTransitionShieldedProofValidationV0};
use crate::execution::validation::state_transition::processor::addresses_minimum_balance::StateTransitionAddressesMinimumBalanceValidationV0;
use crate::execution::validation::state_transition::processor::advanced_structure_with_state::StateTransitionStructureKnownInStateValidationV0;
use crate::execution::validation::state_transition::processor::basic_structure::StateTransitionBasicStructureValidationV0;
use crate::execution::validation::state_transition::processor::identity_balance::StateTransitionIdentityBalanceValidationV0;
use crate::execution::validation::state_transition::processor::identity_based_signature::StateTransitionIdentityBasedSignatureValidationV0;
use crate::execution::validation::state_transition::processor::identity_nonces::{StateTransitionHasIdentityNonceValidationV0, StateTransitionIdentityNonceValidationV0};
use crate::execution::validation::state_transition::processor::is_allowed::StateTransitionIsAllowedValidationV0;
use crate::execution::validation::state_transition::processor::state::StateTransitionStateValidation;
use crate::execution::validation::state_transition::ValidationMode;
use crate::execution::validation::state_transition::processor::traits::address_balances_and_nonces::StateTransitionAddressBalancesAndNoncesValidation;

/// Changes the state transition to the execution event.
/// As this is for check tx it normally does not need to be versioned.
/// We keep the version here just in case for a future radical change.
pub(super) fn state_transition_to_execution_event_for_check_tx_v0<'a, C: CoreRPCLike>(
    platform: &'a PlatformRef<C>,
    state_transition: StateTransition,
    check_tx_level: CheckTxLevel,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<Option<ExecutionEvent<'a>>>, Error> {
    // we need to validate the structure, the fees, and the signature
    let mut state_transition_execution_context =
        StateTransitionExecutionContext::default_for_platform_version(platform_version)?;

    // TODO: There is no point to have it here. There is "_" arm implemented.
    #[allow(unreachable_patterns)]
    match check_tx_level {
        CheckTxLevel::FirstTimeCheck => {
            if state_transition.has_is_allowed_validation()? {
                let result = state_transition.validate_is_allowed(platform, platform_version)?;

                if !result.is_valid() {
                    return Ok(
                        ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_errors(
                            result.errors,
                        ),
                    );
                }
            }

            if state_transition.has_address_witness_validation(platform_version)? {
                let result = state_transition.validate_address_witnesses(
                    &mut state_transition_execution_context,
                    platform_version,
                )?;
                if !result.is_valid() {
                    // If the witnesses are not valid
                    // Proposers should remove such transactions from the block
                    // Other validators should reject blocks with such transactions
                    return Ok(
                        ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_errors(
                            result.errors,
                        ),
                    );
                }
            }

            // Start by validating addresses if the transition has input addresses
            let remaining_address_balances =
                if state_transition.has_addresses_balances_and_nonces_validation() {
                    // Here we validate that all input addresses have enough balance
                    // We also validate that nonces are bumped
                    let result = state_transition.validate_address_balances_and_nonces(
                        platform.drive,
                        &mut state_transition_execution_context,
                        None,
                        platform_version,
                    )?;
                    if !result.is_valid() {
                        // The nonces are not valid or there is not enough balance. The transaction is each replaying an input or there
                        // isn't enough balance, either way the transaction should be rejected.
                        return Ok(
                            ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_errors(
                                result.errors,
                            ),
                        );
                    }
                    Some(result.into_data()?)
                } else {
                    None
                };

            // Only identity top up and identity create do not have nonces validation
            if state_transition.has_identity_nonce_validation(platform_version)? {
                let result = state_transition.validate_identity_nonces(
                    &platform.into(),
                    platform.state.last_block_info(),
                    None,
                    &mut state_transition_execution_context,
                    platform_version,
                )?;

                if !result.is_valid() {
                    return Ok(
                        ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_errors(
                            result.errors,
                        ),
                    );
                }
            }

            // Only Data contract update does not have basic structure validation
            if state_transition.has_basic_structure_validation(platform_version) {
                // First we validate the basic structure
                let result = state_transition
                    .validate_basic_structure(platform.config.network, platform_version)?;

                if !result.is_valid() {
                    return Ok(
                        ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_errors(
                            result.errors,
                        ),
                    );
                }
            }

            // Validate minimum fee for shielded spending transitions (stateless, uses public
            // value_balance). This is cheaper than proof verification so we check it first.
            // Only applies to ShieldedTransfer/Unshield/ShieldedWithdrawal — Shield pays from
            // address inputs and ShieldFromAssetLock pays from the asset lock.
            if state_transition.has_shielded_minimum_fee_validation() {
                let result = state_transition.validate_minimum_shielded_fee(platform_version)?;
                if !result.is_valid() {
                    return Ok(
                        ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_errors(
                            result.errors,
                        ),
                    );
                }
            }

            // Verify ZK proof for shielded transitions (stateless, like signature verification).
            // This happens before any state reads to reject invalid proofs cheaply.
            if state_transition.has_shielded_proof_validation() {
                let result = state_transition.validate_shielded_proof(platform_version)?;
                if !result.is_valid() {
                    return Ok(
                        ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_errors(
                            result.errors,
                        ),
                    );
                }
            }

            // Only identity create does not use identity in state validation, because it doesn't yet have the identity in state
            let mut maybe_identity = if state_transition.uses_identity_in_state() {
                // Validating signature for identity based state transitions (all those except identity create and identity top up)
                // As we already have removed identity create above, it just splits between identity top up (below - false) and
                // all other state transitions (above - true)
                let result = if state_transition.validates_signature_based_on_identity_info() {
                    state_transition.validate_identity_signed_state_transition(
                        platform.drive,
                        None,
                        &mut state_transition_execution_context,
                        platform_version,
                    )
                } else {
                    state_transition.retrieve_identity_info(
                        platform.drive,
                        None,
                        &mut state_transition_execution_context,
                        platform_version,
                    )
                }?;
                if !result.is_valid() {
                    // If the signature is not valid or if we could not retrieve identity info
                    // we do not have the user pay for the state transition.
                    // Since it is most likely not from them
                    // Proposers should remove such transactions from the block
                    // Other validators should reject blocks with such transactions
                    return Ok(
                        ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_errors(
                            result.errors,
                        ),
                    );
                }
                Some(result.into_data()?)
            } else {
                None
            };

            // For identity credit withdrawal and identity credit transfers we have a balance pre check that includes a
            // processing amount and the transfer amount.
            // For other state transitions we only check a min balance for an amount set per version.
            // This is not done for identity create and identity top up who don't have this check here
            if state_transition.has_identity_minimum_balance_pre_check_validation() {
                // Validating that we have sufficient balance for a transfer or withdrawal,
                // this must happen after validating the signature
                let identity =
                    maybe_identity
                        .as_mut()
                        .ok_or(ProtocolError::CorruptedCodeExecution(
                            "identity must be known to validate the balance".to_string(),
                        ))?;

                let result = state_transition
                    .validate_identity_minimum_balance_pre_check(identity, platform_version)?;

                if !result.is_valid() {
                    return Ok(
                        ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_errors(
                            result.errors,
                        ),
                    );
                }
            }

            // For address-based state transitions that transfer or withdraw, we have a balance pre-check
            // that validates addresses have enough remaining balance after the input amounts to cover fees.
            if state_transition.has_addresses_minimum_balance_pre_check_validation() {
                // Validating that addresses have sufficient remaining balance for fees,
                // this must happen after validating the address balances and nonces

                let address_balances = remaining_address_balances.as_ref().ok_or(
                    ProtocolError::CorruptedCodeExecution(
                        "address balances must be known to validate the minimum balance"
                            .to_string(),
                    ),
                )?;
                let result = state_transition.validate_addresses_minimum_balance_pre_check(
                    address_balances,
                    platform_version,
                )?;

                if !result.is_valid() {
                    return Ok(
                        ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_errors(
                            result.errors,
                        ),
                    );
                }
            }

            let action = if state_transition
                .requires_advanced_structure_validation_with_state_on_check_tx()
            {
                let state_transition_action_result = state_transition.transform_into_action(
                    platform,
                    platform.state.last_block_info(),
                    &remaining_address_balances,
                    ValidationMode::CheckTx,
                    &mut state_transition_execution_context,
                    None,
                )?;
                if !state_transition_action_result.is_valid_with_data() {
                    return Ok(
                        ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_errors(
                            state_transition_action_result.errors,
                        ),
                    );
                }
                let action = state_transition_action_result.into_data()?;

                // Validating structure
                let result = state_transition.validate_advanced_structure_from_state(
                    platform.state.last_block_info(),
                    platform.config.network,
                    &action,
                    maybe_identity.as_ref(),
                    &mut state_transition_execution_context,
                    platform_version,
                )?;

                if !result.is_valid() {
                    return Ok(
                        ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_errors(
                            result.errors,
                        ),
                    );
                }
                Some(action)
            } else {
                None
            };

            let action = if state_transition.validates_full_state_on_check_tx() {
                // Validating structure
                let result = state_transition.validate_state(
                    action,
                    platform,
                    ValidationMode::CheckTx,
                    platform.state.last_block_info(),
                    &mut state_transition_execution_context,
                    None,
                )?;

                if !result.is_valid() {
                    return Ok(
                        ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_errors(
                            result.errors,
                        ),
                    );
                }
                result.data
            } else {
                None
            };

            let action = if let Some(action) = action {
                action
            } else {
                let state_transition_action_result = state_transition.transform_into_action(
                    platform,
                    platform.state.last_block_info(),
                    &remaining_address_balances,
                    ValidationMode::CheckTx,
                    &mut state_transition_execution_context,
                    None,
                )?;
                if !state_transition_action_result.is_valid_with_data() {
                    return Ok(
                        ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_errors(
                            state_transition_action_result.errors,
                        ),
                    );
                }
                state_transition_action_result.into_data()?
            };

            let execution_event = ExecutionEvent::create_from_state_transition_action(
                action,
                maybe_identity,
                platform.state.last_committed_block_epoch_ref(),
                state_transition_execution_context,
                platform_version,
            )?;

            Ok(
                ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_data(Some(
                    execution_event,
                )),
            )
        }
        CheckTxLevel::Recheck => {
            if let Some(asset_lock_proof) = state_transition.optional_asset_lock_proof() {
                let mut signable_bytes_hasher =
                    SignableBytesHasher::Bytes(state_transition.signable_bytes()?);
                // we should check that the asset lock is still valid
                let validation_result = asset_lock_proof
                    .verify_is_not_spent_and_has_enough_balance(
                        platform,
                        &mut signable_bytes_hasher,
                        state_transition
                            .required_asset_lock_balance_for_processing_start(platform_version)?,
                        None,
                        platform_version,
                    )?;

                if validation_result.is_valid() {
                    Ok(ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_data(None))
                } else {
                    Ok(
                        ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_errors(
                            validation_result.errors,
                        ),
                    )
                }
            } else {
                // Start by validating addresses if the transition has input addresses
                let remaining_address_balances =
                    if state_transition.has_addresses_balances_and_nonces_validation() {
                        // Here we validate that all input addresses have enough balance
                        // We also validate that nonces are bumped
                        let result = state_transition.validate_address_balances_and_nonces(
                            platform.drive,
                            &mut state_transition_execution_context,
                            None,
                            platform_version,
                        )?;
                        if !result.is_valid() {
                            // The nonces are not valid or there is not enough balance. The transaction is each replaying an input or there
                            // isn't enough balance, either way the transaction should be rejected.
                            return Ok(
                            ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_errors(
                                result.errors,
                            ),
                        );
                        }
                        Some(result.into_data()?)
                    } else {
                        None
                    };

                if state_transition.has_identity_nonce_validation(platform_version)? {
                    let result = state_transition.validate_identity_nonces(
                        &platform.into(),
                        platform.state.last_block_info(),
                        None,
                        &mut state_transition_execution_context,
                        platform_version,
                    )?;

                    if !result.is_valid() {
                        return Ok(
                            ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_errors(
                                result.errors,
                            ),
                        );
                    }
                }

                let state_transition_action_result = state_transition.transform_into_action(
                    platform,
                    platform.state.last_block_info(),
                    &remaining_address_balances,
                    ValidationMode::RecheckTx,
                    &mut state_transition_execution_context,
                    None,
                )?;

                if !state_transition_action_result.is_valid_with_data() {
                    return Ok(
                        ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_errors(
                            state_transition_action_result.errors,
                        ),
                    );
                }
                let action = state_transition_action_result.into_data()?;

                let maybe_identity = if state_transition.uses_identity_in_state() {
                    if let Some(owner_id) = state_transition.owner_id() {
                        platform.drive.fetch_identity_with_balance(
                            owner_id.to_buffer(),
                            None,
                            platform_version,
                        )?
                    } else {
                        None
                    }
                } else {
                    None
                };

                let execution_event = ExecutionEvent::create_from_state_transition_action(
                    action,
                    maybe_identity,
                    platform.state.last_committed_block_epoch_ref(),
                    state_transition_execution_context,
                    platform_version,
                )?;

                Ok(
                    ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_data(Some(
                        execution_event,
                    )),
                )
            }
        }
        _ => Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
            "CheckTxLevel must be first time check or recheck",
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::check_tx::CheckTxLevel;
    use crate::execution::validation::state_transition::state_transitions::tests::setup_identity;
    use crate::platform_types::platform::PlatformRef;
    use crate::platform_types::platform_state::PlatformStateV0Methods;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::dash_to_credits;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::config::DataContractConfig;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::data_contract_create_transition::methods::DataContractCreateTransitionMethodsV0;
    use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
    use dpp::tests::json_document::json_document_to_contract_with_ids;
    use dpp::version::DefaultForPlatformVersion;
    use platform_version::version::PlatformVersion;

    /// Helper to build a signed DataContractCreate StateTransition and return it alongside
    /// the platform instance. This keeps individual tests concise.
    fn setup_data_contract_create_transition(
        credits: dpp::fee::Credits,
    ) -> (
        crate::test::helpers::setup::TempPlatform<MockCoreRPCLike>,
        StateTransition,
    ) {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let (identity, signer, key) = setup_identity(&mut platform, 958, credits);

        let mut data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/dpns/dpns-contract-contested-unique-index.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get contract");

        // Upgrade config to V1 (required since protocol version 12)
        data_contract
            .set_config(DataContractConfig::default_for_version(platform_version).unwrap());

        let data_contract_create_transition = DataContractCreateTransition::new_from_data_contract(
            data_contract,
            1,
            &identity.into_partial_identity_info(),
            key.id(),
            &signer,
            platform_version,
            None,
        )
        .expect("expected to create transition");

        let state_transition: StateTransition = data_contract_create_transition.into();

        (platform, state_transition)
    }

    mod first_time_check {
        use super::*;

        #[test]
        fn should_return_valid_result_for_data_contract_create() {
            let platform_version = PlatformVersion::latest();
            let (platform, state_transition) =
                setup_data_contract_create_transition(dash_to_credits!(2.0));

            let platform_state = platform.state.load();
            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &platform_state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            let result = state_transition_to_execution_event_for_check_tx_v0(
                &platform_ref,
                state_transition,
                CheckTxLevel::FirstTimeCheck,
                platform_version,
            );

            assert!(result.is_ok(), "should not return an error");
            let validation_result = result.unwrap();
            assert!(
                validation_result.is_valid(),
                "validation should succeed: {:?}",
                validation_result.errors
            );
            // For FirstTimeCheck, the execution event should be Some(Some(...))
            assert!(validation_result.data.is_some());
            assert!(validation_result.data.unwrap().is_some());
        }

        #[test]
        fn should_return_invalid_result_for_insufficient_balance() {
            let platform_version = PlatformVersion::latest();
            // Give the identity very little balance -- not enough for the data contract create.
            // The minimum balance pre-check should reject this.
            let (platform, state_transition) = setup_data_contract_create_transition(1);

            let platform_state = platform.state.load();
            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &platform_state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            let result = state_transition_to_execution_event_for_check_tx_v0(
                &platform_ref,
                state_transition,
                CheckTxLevel::FirstTimeCheck,
                platform_version,
            );

            assert!(
                result.is_ok(),
                "should not return an Error, just an invalid validation result"
            );
            let validation_result = result.unwrap();
            assert!(
                !validation_result.is_valid(),
                "validation should fail for insufficient balance"
            );
        }

        #[test]
        fn should_return_invalid_result_for_bad_signature() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(2.0));

            let mut data_contract = json_document_to_contract_with_ids(
                "tests/supporting_files/contract/dpns/dpns-contract-contested-unique-index.json",
                None,
                None,
                false,
                platform_version,
            )
            .expect("expected to get contract");

            // Upgrade config to V1 (required since protocol version 12)
            data_contract
                .set_config(DataContractConfig::default_for_version(platform_version).unwrap());

            let data_contract_create_transition =
                DataContractCreateTransition::new_from_data_contract(
                    data_contract,
                    1,
                    &identity.into_partial_identity_info(),
                    key.id(),
                    &signer,
                    platform_version,
                    None,
                )
                .expect("expected to create transition");

            let mut state_transition: StateTransition = data_contract_create_transition.into();

            // Corrupt the signature to make it invalid
            state_transition.set_signature(dpp::platform_value::BinaryData::new(vec![0u8; 65]));

            let platform_state = platform.state.load();
            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &platform_state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            let result = state_transition_to_execution_event_for_check_tx_v0(
                &platform_ref,
                state_transition,
                CheckTxLevel::FirstTimeCheck,
                platform_version,
            );

            assert!(result.is_ok(), "should not return an Error");
            let validation_result = result.unwrap();
            assert!(
                !validation_result.is_valid(),
                "validation should fail for bad signature"
            );
        }

        #[test]
        fn should_return_invalid_result_for_replayed_nonce() {
            let platform_version = PlatformVersion::latest();
            let (platform, state_transition) =
                setup_data_contract_create_transition(dash_to_credits!(2.0));

            let serialized = state_transition
                .serialize_to_bytes()
                .expect("expected to serialize");

            let platform_state = platform.state.load();

            // First, process the transition to consume the nonce
            let transaction = platform.drive.grove.start_transaction();
            platform
                .platform
                .process_raw_state_transitions(
                    std::slice::from_ref(&serialized),
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            // Now try the same transition again -- the nonce should be invalid
            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &platform_state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            let result = state_transition_to_execution_event_for_check_tx_v0(
                &platform_ref,
                state_transition,
                CheckTxLevel::FirstTimeCheck,
                platform_version,
            );

            assert!(result.is_ok(), "should not return an Error");
            let validation_result = result.unwrap();
            assert!(
                !validation_result.is_valid(),
                "should fail because the nonce has already been used"
            );
            assert!(
                validation_result.errors.iter().any(|e| matches!(
                    e,
                    ConsensusError::StateError(StateError::InvalidIdentityNonceError(_))
                )),
                "expected InvalidIdentityNonceError but got: {:?}",
                validation_result.errors
            );
        }
    }

    mod recheck {
        use super::*;

        #[test]
        fn should_return_valid_result_for_data_contract_create_recheck() {
            let platform_version = PlatformVersion::latest();
            let (platform, state_transition) =
                setup_data_contract_create_transition(dash_to_credits!(2.0));

            let platform_state = platform.state.load();
            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &platform_state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            // Recheck for a DataContractCreate: no asset lock proof, has identity nonce,
            // goes through the non-asset-lock recheck path
            let result = state_transition_to_execution_event_for_check_tx_v0(
                &platform_ref,
                state_transition,
                CheckTxLevel::Recheck,
                platform_version,
            );

            assert!(result.is_ok(), "should not return an error");
            let validation_result = result.unwrap();
            assert!(
                validation_result.is_valid(),
                "recheck should pass for valid transition: {:?}",
                validation_result.errors
            );
            assert!(
                validation_result.data.is_some(),
                "should return an execution event for recheck"
            );
        }

        #[test]
        fn should_return_invalid_result_for_replayed_nonce_on_recheck() {
            let platform_version = PlatformVersion::latest();
            let (platform, state_transition) =
                setup_data_contract_create_transition(dash_to_credits!(2.0));

            let serialized = state_transition
                .serialize_to_bytes()
                .expect("expected to serialize");

            let platform_state = platform.state.load();

            // Process the transition first to consume the nonce
            let transaction = platform.drive.grove.start_transaction();
            platform
                .platform
                .process_raw_state_transitions(
                    std::slice::from_ref(&serialized),
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            // Recheck should fail because the nonce is spent
            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &platform_state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            let result = state_transition_to_execution_event_for_check_tx_v0(
                &platform_ref,
                state_transition,
                CheckTxLevel::Recheck,
                platform_version,
            );

            assert!(result.is_ok(), "should not return an Error");
            let validation_result = result.unwrap();
            assert!(
                !validation_result.is_valid(),
                "recheck should fail because nonce was consumed"
            );
            assert!(
                validation_result.errors.iter().any(|e| matches!(
                    e,
                    ConsensusError::StateError(StateError::InvalidIdentityNonceError(_))
                )),
                "expected InvalidIdentityNonceError but got: {:?}",
                validation_result.errors
            );
        }
    }
}
