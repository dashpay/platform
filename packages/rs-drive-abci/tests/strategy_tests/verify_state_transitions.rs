use dpp::address_funds::PlatformAddress;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::document::{Document, DocumentV0Getters};
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValueGettersV0;
use dpp::document::property_names::PRICE;
use dpp::state_transition::proof_result::{StateTransitionProofOutcome, StateTransitionProofResult};
use dpp::state_transition::{StateTransition };
use dpp::state_transition::StateTransitionType;
use dpp::state_transition::StateTransitionWitnessSigned;
use dpp::state_transition::address_funding_from_asset_lock_transition::accessors::AddressFundingFromAssetLockTransitionAccessorsV0;
use dpp::state_transition::identity_create_from_addresses_transition::accessors::IdentityCreateFromAddressesTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_to_addresses_transition::accessors::IdentityCreditTransferToAddressesTransitionAccessorsV0;
use dpp::state_transition::identity_topup_from_addresses_transition::accessors::IdentityTopUpFromAddressesTransitionAccessorsV0;
use dpp::version::PlatformVersion;
use drive::drive::identity::key::fetch::IdentityKeysRequest;
use drive::drive::Drive;
use drive::error::proof::ProofError;
use drive::error::Error as DriveError;
use drive::query::{SingleDocumentDriveQuery, SingleDocumentDriveQueryContestedStatus};
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use drive::state_transition_action::StateTransitionAction;
use drive_abci::execution::validation::state_transition::transformer::StateTransitionActionTransformer;
use drive_abci::platform_types::platform::PlatformRef;
use drive_abci::rpc::core::MockCoreRPCLike;
use tenderdash_abci::proto::abci::ExecTxResult;
use dapi_grpc::drive::v0::GetProofsRequest;
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
use dpp::data_contracts::SystemDataContract;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::serialization::PlatformSerializable;
use dpp::prelude::AddressNonce;
use dpp::voting::votes::Vote;
use drive::drive::votes::resolved::vote_polls::ResolvedVotePoll;
use drive::drive::votes::resolved::votes::resolved_resource_vote::accessors::v0::ResolvedResourceVoteGettersV0;
use drive::drive::votes::resolved::votes::ResolvedVote;
use drive::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::{DocumentCreateTransitionActionAccessorsV0, DocumentFromCreateTransitionAction};
use drive::state_transition_action::batch::batched_transition::document_transition::document_purchase_transition_action::DocumentPurchaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_replace_transition_action::DocumentFromReplaceTransitionAction;
use drive::state_transition_action::batch::batched_transition::document_transition::document_transfer_transition_action::DocumentTransferTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_update_price_transition_action::DocumentUpdatePriceTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use drive_abci::abci::app::FullAbciApplication;
use drive_abci::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use drive_abci::execution::validation::state_transition::ValidationMode;
use drive_abci::platform_types::platform_state::PlatformStateV0Methods;
use platform_version::DefaultForPlatformVersion;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, OnceLock};

static STATE_TRANSITION_TYPE_COUNTER: OnceLock<Mutex<BTreeMap<String, usize>>> = OnceLock::new();

fn state_transition_counter() -> &'static Mutex<BTreeMap<String, usize>> {
    STATE_TRANSITION_TYPE_COUNTER.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn track_state_transition_type(transition_type: StateTransitionType) {
    let counter = state_transition_counter();
    let mut guard = counter
        .lock()
        .expect("state transition counter lock should not be poisoned");
    *guard.entry(transition_type.to_string()).or_insert(0) += 1;
}

fn log_state_transition_type_statistics() {
    let counter = state_transition_counter();
    let guard = counter
        .lock()
        .expect("state transition counter lock should not be poisoned");

    if guard.is_empty() {
        println!("state transition type stats: no state transitions tracked");
    } else {
        println!("state transition type stats: ");
        for (transition_type, count) in guard.iter() {
            println!("STATE_TRANSITION:  {transition_type}:{count}");
        }
        println!();
    }
}

fn latest_block_info<C>(platform: &PlatformRef<C>) -> BlockInfo {
    platform
        .state
        .last_committed_block_info()
        .as_ref()
        .map(|info| info.basic_info().clone())
        .unwrap_or_else(BlockInfo::default)
}

fn assert_transition_yields_affected_state_snapshot(
    state_transition: &StateTransition,
    block_info: &BlockInfo,
    proof: &[u8],
    operation: &str,
    platform_version: &PlatformVersion,
) {
    let (_root_hash, outcome) = Drive::verify_state_transition_was_executed_with_proof(
        state_transition,
        block_info,
        proof,
        &|_| Ok(None),
        platform_version,
    )
    .unwrap_or_else(|error| {
        panic!("{operation}: affected-state verification should succeed: {error:?}")
    });

    // The post-state proof cannot be bound to this transition's execution,
    // so the outcome must stay tagged as an affected-state snapshot.
    assert!(
        matches!(outcome, StateTransitionProofOutcome::AffectedState(_)),
        "{operation}: post-state proof must not be treated as execution evidence, got {outcome:?}"
    );
}

fn assert_address_inputs_state(
    transition_inputs: &BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    action_inputs: &BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    proof_address_infos: &BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>>,
    context: &str,
) {
    assert_eq!(
        action_inputs.len(),
        transition_inputs.len(),
        "{context}: action inputs length mismatch"
    );

    for (address, (transition_nonce, _)) in transition_inputs {
        let Some(&(action_nonce, action_balance)) = action_inputs.get(address) else {
            panic!("{context}: action missing address {address:?}");
        };
        let Some(Some((proof_nonce, proof_balance))) = proof_address_infos.get(address) else {
            panic!("{context}: missing proof info for address {address:?}");
        };

        assert!(
            *proof_nonce >= *transition_nonce,
            "{context}: proof nonce {proof_nonce} should be >= transition nonce {transition_nonce} for address {address:?}"
        );

        assert!(
            action_nonce >= *transition_nonce,
            "{context}: action nonce {action_nonce} should be >= transition nonce {transition_nonce} for address {address:?}"
        );

        assert_eq!(
            *proof_nonce, action_nonce,
            "{context}: proof nonce mismatch for address {address:?}"
        );

        assert_eq!(
            *proof_balance, action_balance,
            "{context}: proof/action balance mismatch for address {address:?}"
        );
    }
}

/// Verifies a single optional output, ensuring the action's address/balance
/// (if any) matches the transition expectation and the proof snapshot.
fn assert_optional_action_output_state(
    transition_output: Option<(PlatformAddress, Credits)>,
    action_output: Option<(PlatformAddress, Credits)>,
    proof_address_infos: &BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>>,
    context: &str,
) {
    match (transition_output, action_output) {
        (
            Some((transition_address, transition_balance)),
            Some((action_address, action_balance)),
        ) => {
            assert_eq!(
                transition_address, action_address,
                "{context}: action output address mismatch (expected {:?})",
                transition_address
            );
            assert!(
                action_balance >= transition_balance,
                "{context}: action output balance {} should be >= transition balance {} for address {:?}",
                action_balance,
                transition_balance,
                transition_address
            );
            let Some(Some((_, proof_balance))) = proof_address_infos.get(&action_address) else {
                panic!(
                    "{context}: missing proof info for output address {:?}",
                    action_address
                );
            };
            assert_eq!(
                *proof_balance, action_balance,
                "{context}: proof balance mismatch for output address {:?}",
                action_address
            );
        }
        (None, None) => {}
        (Some((transition_address, _)), None) => {
            panic!(
                "{context}: missing action output for transition output address {:?}",
                transition_address
            );
        }
        (None, Some((action_address, _))) => {
            panic!(
                "{context}: unexpected action output for address {:?}",
                action_address
            );
        }
    }
}

/// Handles asset lock outputs where each address may either specify an
/// explicit payout or only a remainder resolved later, validating both the
/// optional amounts and the resolved balances against proofs.
fn assert_asset_lock_outputs_state(
    transition_outputs: &BTreeMap<PlatformAddress, Option<Credits>>,
    action_outputs: &BTreeMap<PlatformAddress, Option<Credits>>,
    context: &str,
) {
    assert_eq!(
        transition_outputs.len(),
        action_outputs.len(),
        "{context}: action/transition output count mismatch"
    );

    for (address, transition_value) in transition_outputs {
        let Some(action_value) = action_outputs.get(address) else {
            panic!("{context}: missing action output for address {:?}", address);
        };
        match (transition_value, action_value) {
            (Some(transition_balance), Some(action_balance)) => assert!(
                *action_balance >= *transition_balance,
                "{context}: action explicit output {} should be >= transition output {} for address {:?}",
                action_balance,
                transition_balance,
                address
            ),
            (None, None) => {}
            (Some(_), None) => panic!(
                "{context}: missing action output amount for address {:?}",
                address
            ),
            (None, Some(_)) => panic!(
                "{context}: unexpected explicit action output for remainder address {:?}",
                address
            ),
        }
    }
}

pub(crate) fn verify_state_transitions_were_or_were_not_executed(
    abci_app: &FullAbciApplication<MockCoreRPCLike>,
    expected_root_hash: &[u8; 32],
    state_transitions: &[(StateTransition, ExecTxResult)],
    expected_validation_errors: &[u32],
    platform_version: &PlatformVersion,
) -> bool {
    let state = abci_app.platform.state.load();
    let platform = PlatformRef {
        drive: &abci_app.platform.drive,
        state: &state,
        config: &abci_app.platform.config,
        core_rpc: &abci_app.platform.core_rpc,
    };

    //actions are easier to transform to queries
    let actions = state_transitions
        .iter()
        .enumerate()
        .map(|(num, (state_transition, result))| {
            // if let StateTransition::DocumentsBatch(batch) = state_transition {
            //     let _first = batch.document_transitions().first().unwrap();
            //
            //     // dbg!(batch.transitions().len(), hex::encode(first.base().id()), state.height(), first.to_string());
            // }

            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("expected to get an execution context");

            // Start by validating addresses if the transition has input addresses
            let remaining_address_input_balances = if let Some(inputs) = state_transition.inputs() {
                let fetched_balances = platform
                    .drive
                    .fetch_balances_with_nonces(inputs.keys(), None, platform_version)
                    .expect("expected to fetch current balances with current nonces");
                let balances = fetched_balances
                    .into_iter()
                    .map(|(address, maybe_found)| {
                        let (nonce, balance) =
                            maybe_found.expect("expected address to have balance");
                        (address, (nonce, balance))
                    })
                    .collect();
                Some(balances)
            } else {
                None
            };

            let consensus_validation_result = match state_transition.transform_into_action(
                &platform,
                abci_app.platform.state.load().last_block_info(),
                &remaining_address_input_balances,
                ValidationMode::NoValidation, //using check_tx so we don't validate state
                &mut execution_context,
                None,
            ) {
                Ok(consensus_validation_result) => consensus_validation_result,
                Err(e) => {
                    if expected_validation_errors.contains(&result.code) {
                        return (state_transition.clone(), None, false);
                    } else {
                        panic!("{}", e)
                    }
                }
            };

            if !consensus_validation_result.is_valid() {
                panic!(
                    "expected state transition {:?} to be valid, errors are {:?}",
                    num, consensus_validation_result.errors
                )
            }
            let action = consensus_validation_result.into_data().unwrap_or_else(|_| {
                panic!(
                    "expected state transitions to be valid {:?}",
                    state_transition
                )
            });
            (state_transition.clone(), Some(action), result.code == 0)
        })
        .collect::<Vec<_>>();

    for (state_transition, action, was_executed) in &actions {
        let transition_type = state_transition.state_transition_type();
        tracing::debug!(
            %transition_type,
            ?state_transition,
            ?action,
            was_executed,
            "Verifying state transition execution"
        );
        track_state_transition_type(transition_type);

        let state_transition_bytes = state_transition
            .serialize_to_bytes()
            .expect("serialize state transition");

        let request = GetProofsRequest {
            state_transition: state_transition_bytes,
        };

        let result = abci_app
            .platform
            .query_proofs(request, &state, platform_version)
            .expect(&format!(
                "proof query for {} should succeed",
                transition_type
            ));

        if !result.is_valid() {
            panic!(
                "expected to get a valid proof response, got errors {:?}",
                result.errors
            )
        }

        let response = result.into_data().expect("get proof response");

        let response_proof = response.proof.expect("existing proof");

        if let Some(action) = action {
            match action {
                StateTransitionAction::DataContractCreateAction(data_contract_create) => {
                    // let fetched_contract = abci_app
                    //     .platform.drive.fetch_contract(data_contract_create.data_contract_ref().id().into_buffer(), None, None, None, platform_version).unwrap().unwrap();
                    // we expect to get an identity that matches the state transition
                    let keeps_history = data_contract_create
                        .data_contract_ref()
                        .config()
                        .keeps_history();
                    let (root_hash, contract) = Drive::verify_contract(
                        &response_proof.grovedb_proof,
                        Some(keeps_history),
                        false,
                        true,
                        data_contract_create.data_contract_ref().id().into_buffer(),
                        platform_version,
                    )
                    .expect("expected to verify contract");
                    assert_eq!(
                        &root_hash,
                        expected_root_hash,
                        "state last block info {:?}",
                        platform.state.last_committed_block_info()
                    );

                    if *was_executed {
                        assert_eq!(
                            &contract.expect("expected a contract"),
                            data_contract_create.data_contract_ref(),
                        )
                    } else {
                        //there is the possibility that the state transition was not executed because it already existed,
                        // we can discount that for now in tests
                        assert!(contract.is_none())
                    }
                }
                StateTransitionAction::DataContractUpdateAction(data_contract_update) => {
                    // we expect to get an identity that matches the state transition
                    let keeps_history = data_contract_update
                        .data_contract_ref()
                        .config()
                        .keeps_history();
                    let (root_hash, contract) = Drive::verify_contract(
                        &response_proof.grovedb_proof,
                        Some(keeps_history),
                        false,
                        true,
                        data_contract_update.data_contract_ref().id().into_buffer(),
                        platform_version,
                    )
                    .expect("expected to verify contract");
                    assert_eq!(
                        &root_hash,
                        expected_root_hash,
                        "state last block info {:?}",
                        platform.state.last_committed_block_info()
                    );
                    if *was_executed {
                        assert!(contract
                            .expect("expected a contract")
                            .equal_ignoring_time_fields(
                                data_contract_update.data_contract_ref(),
                                platform_version
                            )
                            .expect("expected to be able to check equality"),);
                    } else if contract.is_some() {
                        //there is the possibility that the state transition was not executed and the state is equal to the previous
                        // state, aka there would have been no change anyways, we can discount that for now
                        assert_ne!(
                            &contract.expect("expected a contract"),
                            data_contract_update.data_contract_ref(),
                        );
                    }
                }
                StateTransitionAction::BatchAction(batch_transition) => {
                    if batch_transition.transitions().is_empty() {
                        panic!("we should have at least one transition");
                    }

                    let Some(transition_action) = batch_transition.transitions().first() else {
                        panic!("we should have at least one transition");
                    };
                    match transition_action {
                        BatchedTransitionAction::DocumentAction(document_action) => {
                            let contract_fetch_info =
                                document_action.base().data_contract_fetch_info();

                            let document_type = contract_fetch_info
                                .contract
                                .document_type_for_name(
                                    document_action.base().document_type_name().as_str(),
                                )
                                .expect("get document type");
                            let contested_status =
                                if let DocumentTransitionAction::CreateAction(create_action) =
                                    document_action
                                {
                                    if create_action.prefunded_voting_balance().is_some() {
                                        SingleDocumentDriveQueryContestedStatus::Contested
                                    } else {
                                        SingleDocumentDriveQueryContestedStatus::NotContested
                                    }
                                } else {
                                    SingleDocumentDriveQueryContestedStatus::NotContested
                                };

                            let query = SingleDocumentDriveQuery {
                                contract_id: document_action
                                    .base()
                                    .data_contract_id()
                                    .into_buffer(),
                                document_type_name: document_action
                                    .base()
                                    .document_type_name()
                                    .clone(),
                                document_type_keeps_history: document_type.documents_keep_history(),
                                document_id: document_action.base().id().into_buffer(),
                                block_time_ms: None, //None because we want latest
                                contested_status,
                            };

                            // dbg!(
                            //     platform.state.height(),
                            //     document_transition_action.action_type(),
                            //     document_transition_action
                            //         .base()
                            //         .id()
                            //         .to_string(Encoding::Base58)
                            // );

                            let (root_hash, document) = query
                                .verify_proof(
                                    false,
                                    &response_proof.grovedb_proof,
                                    document_type,
                                    platform_version,
                                )
                                .expect("expected to verify a document");

                            assert_eq!(
                                &root_hash,
                                expected_root_hash,
                                "state last block info {:?}",
                                platform.state.last_committed_block_info()
                            );

                            match document_action {
                                DocumentTransitionAction::CreateAction(creation_action) => {
                                    if *was_executed {
                                        let document = document.unwrap_or_else(|| {
                                            panic!(
                                                "expected a document on block {}",
                                                platform.state.last_committed_block_height()
                                            )
                                        });
                                        // dbg!(
                                        //     &document,
                                        //     Document::try_from_create_transition(
                                        //         creation_action,
                                        //         documents_batch_transition.owner_id(),
                                        //         platform_version,
                                        //     )
                                        //     .expect("expected to get document")
                                        // );
                                        assert_eq!(
                                            document,
                                            Document::try_from_create_transition_action(
                                                creation_action,
                                                batch_transition.owner_id(),
                                                platform_version,
                                            )
                                            .expect("expected to get document")
                                        );
                                    } else {
                                        //there is the possibility that the state transition was not executed because it already existed,
                                        // we can discount that for now in tests
                                        assert!(document.is_none());
                                    }
                                }
                                DocumentTransitionAction::ReplaceAction(replace_action) => {
                                    if *was_executed {
                                        // it's also possible we deleted something we replaced
                                        if let Some(document) = document {
                                            assert_eq!(
                                                document,
                                                Document::try_from_replace_transition_action(
                                                    replace_action,
                                                    batch_transition.owner_id(),
                                                    platform_version,
                                                )
                                                .expect("expected to get document")
                                            );
                                        }
                                    } else {
                                        //there is the possibility that the state transition was not executed and the state is equal to the previous
                                        // state, aka there would have been no change anyways, we can discount that for now
                                        if let Some(document) = document {
                                            assert_ne!(
                                                document,
                                                Document::try_from_replace_transition_action(
                                                    replace_action,
                                                    batch_transition.owner_id(),
                                                    platform_version,
                                                )
                                                .expect("expected to get document")
                                            );
                                        }
                                    }
                                }

                                DocumentTransitionAction::DeleteAction(_) => {
                                    // we expect no document
                                    assert!(document.is_none());
                                }
                                DocumentTransitionAction::TransferAction(transfer_action) => {
                                    if *was_executed {
                                        // it's also possible we deleted something we replaced
                                        if let Some(document) = document {
                                            assert_eq!(
                                                document.owner_id(),
                                                transfer_action.document().owner_id()
                                            );
                                        }
                                    } else {
                                        //there is the possibility that the state transition was not executed and the state is equal to the previous
                                        // state, aka there would have been no change anyways, we can discount that for now
                                        if let Some(document) = document {
                                            assert_ne!(
                                                document.owner_id(),
                                                transfer_action.document().owner_id()
                                            );
                                        }
                                    }
                                }
                                DocumentTransitionAction::PurchaseAction(purchase_action) => {
                                    if *was_executed {
                                        if let Some(document) = document {
                                            assert_eq!(
                                                document.owner_id(),
                                                purchase_action.document().owner_id()
                                            );
                                        }
                                    } else {
                                        //there is the possibility that the state transition was not executed and the state is equal to the previous
                                        // state, aka there would have been no change anyways, we can discount that for now
                                        if let Some(document) = document {
                                            assert_ne!(
                                                document.owner_id(),
                                                purchase_action.document().owner_id()
                                            );
                                        }
                                    }
                                }
                                DocumentTransitionAction::UpdatePriceAction(
                                    update_price_action,
                                ) => {
                                    if *was_executed {
                                        if let Some(document) = document {
                                            assert_eq!(
                                                document.get(PRICE),
                                                update_price_action.document().get(PRICE)
                                            );
                                        }
                                    } else {
                                        //there is the possibility that the state transition was not executed and the state is equal to the previous
                                        // state, aka there would have been no change anyways, we can discount that for now
                                        if let Some(document) = document {
                                            assert_ne!(
                                                document.get(PRICE),
                                                update_price_action.document().get(PRICE)
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        BatchedTransitionAction::TokenAction(token_transition_action) => {
                            if token_transition_action
                                .keeps_history()
                                .expect("expected no error in token action keeps history")
                            {
                                let token_id = token_transition_action.base().token_id();
                                let document_type_name = token_transition_action
                                    .historical_document_type_name()
                                    .to_string();

                                let token_history = platform
                                    .drive
                                    .cache
                                    .system_data_contracts
                                    .load_token_history(platform_version)
                                    .expect("expected the token_history system contract");

                                let query = SingleDocumentDriveQuery {
                                    contract_id: SystemDataContract::TokenHistory.id().to_buffer(),
                                    document_type_name,
                                    document_type_keeps_history: false,
                                    document_id: token_transition_action
                                        .historical_document_id(batch_transition.owner_id())
                                        .to_buffer(),
                                    block_time_ms: None, //None because we want latest
                                    contested_status:
                                        SingleDocumentDriveQueryContestedStatus::NotContested,
                                };

                                let (root_hash, serialized_document) = query
                                    .verify_proof_keep_serialized(
                                        false,
                                        &response_proof.grovedb_proof,
                                        platform_version,
                                    )
                                    .expect("expected to verify a document");

                                assert_eq!(
                                    &root_hash,
                                    expected_root_hash,
                                    "state last block info {:?}",
                                    platform.state.last_committed_block_info()
                                );

                                assert!(
                                    serialized_document.is_some(),
                                    "we expect a token history document"
                                );

                                let expected_document = token_transition_action
                                    .build_historical_document(
                                        token_id,
                                        batch_transition.owner_id(),
                                        token_transition_action.base().identity_contract_nonce(),
                                        platform
                                            .state
                                            .last_committed_block_info()
                                            .as_ref()
                                            .expect("expected last commited block info")
                                            .basic_info(),
                                        platform_version,
                                    )
                                    .expect("expected to build historical document");

                                let serialized_expected_document = expected_document
                                    .serialize(
                                        token_transition_action
                                            .historical_document_type(&token_history)
                                            .expect("expected document type"),
                                        &token_history,
                                        platform_version,
                                    )
                                    .expect("expected to serialize");

                                assert_eq!(
                                    serialized_document.unwrap(),
                                    serialized_expected_document
                                );
                            } else {
                                todo!();
                            }
                        }
                        BatchedTransitionAction::BumpIdentityDataContractNonce(_) => {
                            panic!("we should not have a bump identity data contract nonce");
                        }
                    }
                }
                StateTransitionAction::IdentityCreateAction(identity_create_transition) => {
                    // we expect to get an identity that matches the state transition
                    let (root_hash, identity) = Drive::verify_full_identity_by_identity_id(
                        &response_proof.grovedb_proof,
                        false,
                        identity_create_transition.identity_id().into_buffer(),
                        platform_version,
                    )
                    .expect("expected to verify full identity");
                    assert_eq!(
                        &root_hash,
                        expected_root_hash,
                        "state last block info {:?}",
                        platform.state.last_committed_block_info()
                    );
                    if *was_executed {
                        // other state transitions might have happened in the same block the identity
                        // was created
                        let proved_identity = identity
                            .expect("expected an identity")
                            .into_partial_identity_info_no_balance();
                        assert_eq!(proved_identity.id, identity_create_transition.identity_id());
                    } else {
                        //there is the possibility that the state transition was not executed because it already existed,
                        // we can discount that for now in tests
                        assert!(identity.is_none());
                    }
                }
                StateTransitionAction::IdentityTopUpAction(identity_top_up_transition) => {
                    // we expect to get an identity that matches the state transition
                    // Use verify_subset_of_proof=true because the response proof is a merged
                    // proof covering both revision (Identities tree) and balance (Balances tree)
                    let (root_hash, balance) = Drive::verify_identity_balance_for_identity_id(
                        &response_proof.grovedb_proof,
                        identity_top_up_transition.identity_id().into_buffer(),
                        true,
                        platform_version,
                    )
                    .expect("expected to verify balance identity for top up");
                    let balance = balance.expect("expected a balance");
                    assert_eq!(
                        &root_hash,
                        expected_root_hash,
                        "state last block info {:?}",
                        platform.state.last_committed_block_info()
                    );

                    if *was_executed {
                        //while this isn't 100% sure to be true (in the case of debt,
                        // for the tests we have we can use it
                        assert!(
                            identity_top_up_transition
                                .top_up_asset_lock_value()
                                .remaining_credit_value()
                                <= balance
                        );
                    }
                }
                StateTransitionAction::IdentityCreditWithdrawalAction(
                    identity_credit_withdrawal_transition,
                ) => {
                    // todo: we should also verify the document
                    // we expect to get an identity that matches the state transition
                    // Use verify_subset_of_proof=true because GroveDB proofs may include
                    // lower layers for sibling subtrees at the root level
                    let (root_hash, balance) = Drive::verify_identity_balance_for_identity_id(
                        &response_proof.grovedb_proof,
                        identity_credit_withdrawal_transition
                            .identity_id()
                            .into_buffer(),
                        true,
                        platform_version,
                    )
                    .expect("expected to verify balance identity for withdrawal");
                    let _balance = balance.expect("expected a balance");
                    assert_eq!(
                        &root_hash,
                        expected_root_hash,
                        "state last block info {:?}",
                        platform.state.last_committed_block_info()
                    );

                    //todo: we need to do more here
                }
                StateTransitionAction::IdentityUpdateAction(identity_update_transition) => {
                    // we expect to get an identity that matches the state transition
                    let (root_hash, identity) = Drive::verify_identity_keys_by_identity_id(
                        &response_proof.grovedb_proof,
                        IdentityKeysRequest::new_all_keys_query(
                            &identity_update_transition.identity_id().into_buffer(),
                            None,
                        ),
                        false,
                        false,
                        false,
                        platform_version,
                    )
                    .expect("expected to verify identity keys");
                    let identity = identity.expect("expected an identity");
                    assert_eq!(
                        &root_hash,
                        expected_root_hash,
                        "state last block info {:?}",
                        platform.state.last_committed_block_info()
                    );
                    // we need to verify that the partial identity has all keys we added
                    let has_all_keys = identity_update_transition
                        .public_keys_to_add()
                        .iter()
                        .all(|added| identity.loaded_public_keys.contains_key(&added.id()));
                    let has_no_removed_key = !identity_update_transition
                        .public_keys_to_disable()
                        .iter()
                        .any(|removed| identity.loaded_public_keys.contains_key(removed));
                    assert!(has_all_keys);
                    assert!(has_no_removed_key);
                }
                StateTransitionAction::IdentityCreditTransferAction(
                    identity_credit_transfer_action,
                ) => {
                    // we expect to get an identity that matches the state transition
                    let (root_hash_identity, _balance_identity) =
                        Drive::verify_identity_balance_for_identity_id(
                            &response_proof.grovedb_proof,
                            identity_credit_transfer_action.identity_id().into_buffer(),
                            true,
                            platform_version,
                        )
                        .expect("expected to verify balance identity for credit transfer");

                    assert_eq!(
                        &root_hash_identity,
                        expected_root_hash,
                        "state last block info {:?}",
                        platform.state.last_committed_block_info()
                    );

                    let (root_hash_recipient, balance_recipient) =
                        Drive::verify_identity_balance_for_identity_id(
                            &response_proof.grovedb_proof,
                            identity_credit_transfer_action.recipient_id().into_buffer(),
                            true,
                            platform_version,
                        )
                        .expect("expected to verify balance recipient");

                    assert_eq!(
                        &root_hash_recipient,
                        expected_root_hash,
                        "state last block info {:?}",
                        platform.state.last_committed_block_info()
                    );

                    if *was_executed {
                        let balance_recipient = balance_recipient.expect("expected a balance");

                        assert!(
                            balance_recipient >= identity_credit_transfer_action.transfer_amount()
                        );
                    }
                }
                StateTransitionAction::MasternodeVoteAction(masternode_vote_action) => {
                    let data_contract = match masternode_vote_action.vote_ref() {
                        ResolvedVote::ResolvedResourceVote(resource_vote) => match resource_vote
                            .vote_poll()
                        {
                            ResolvedVotePoll::ContestedDocumentResourceVotePollWithContractInfo(
                                contested_document_resource_vote_poll,
                            ) => contested_document_resource_vote_poll.contract.as_ref(),
                        },
                    };

                    let vote: Vote = masternode_vote_action.vote_ref().clone().into();

                    // we expect to get a vote that matches the state transition
                    let (root_hash_vote, maybe_vote) = Drive::verify_masternode_vote(
                        &response_proof.grovedb_proof,
                        masternode_vote_action.pro_tx_hash().into_buffer(),
                        &vote,
                        data_contract,
                        false, // we are not in a subset, we have just one vote
                        platform_version,
                    )
                    .expect("expected to verify balance identity");

                    assert_eq!(
                        &root_hash_vote,
                        expected_root_hash,
                        "state last block info {:?}",
                        platform.state.last_committed_block_info()
                    );

                    if *was_executed {
                        let executed_vote = maybe_vote.expect("expected a vote");

                        assert_eq!(&executed_vote, &vote);
                    }
                }
                StateTransitionAction::BumpIdentityNonceAction(_) => {}
                StateTransitionAction::BumpIdentityDataContractNonceAction(_) => {}
                StateTransitionAction::PartiallyUseAssetLockAction(_) => {}
                StateTransitionAction::IdentityCreateFromAddressesAction(
                    identity_create_from_addresses_action,
                ) => {
                    if let StateTransition::IdentityCreateFromAddresses(
                        identity_create_from_addresses_transition,
                    ) = state_transition
                    {
                        if *was_executed {
                            let block_info = latest_block_info(&platform);
                            let (root_hash, proof_result) =
                                Drive::verify_state_transition_was_executed_with_proof(
                                    state_transition,
                                    &block_info,
                                    &response_proof.grovedb_proof,
                                    &|_| Ok(None),
                                    platform_version,
                                )
                                .expect("expected to verify identity create from addresses proof");

                            assert_eq!(
                                &root_hash,
                                expected_root_hash,
                                "state last block info {:?}",
                                platform.state.last_committed_block_info()
                            );

                            let StateTransitionProofOutcome::ExecutionProved(
                                StateTransitionProofResult::VerifiedIdentityFullWithAddressInfos(
                                    proved_identity,
                                    proof_address_infos_map,
                                ),
                            ) = proof_result
                            else {
                                panic!("expected identity/address infos for identity create from addresses proof");
                            };

                            assert_eq!(
                                proved_identity.id(),
                                identity_create_from_addresses_action.identity_id(),
                                "proof identity should match created identity"
                            );
                            tracing::trace!(state_transition=?identity_create_from_addresses_transition,
                                "Checking proof of identity create from addresses transition"
                            );
                            let mut expected_addresses: BTreeSet<PlatformAddress> =
                                identity_create_from_addresses_transition
                                    .inputs()
                                    .keys()
                                    .copied()
                                    .collect();
                            if let Some((change_address, _)) =
                                identity_create_from_addresses_transition.output()
                            {
                                expected_addresses.insert(*change_address);
                            }

                            let proved_addresses: BTreeSet<PlatformAddress> =
                                proof_address_infos_map.keys().copied().collect();

                            assert_eq!(
                                proved_addresses, expected_addresses,
                                "proved addresses should match inputs used to fund identity"
                            );

                            assert_address_inputs_state(
                                identity_create_from_addresses_transition.inputs(),
                                identity_create_from_addresses_action
                                    .inputs_with_remaining_balance(),
                                &proof_address_infos_map,
                                "identity create from addresses",
                            );

                            let transition_change_output =
                                identity_create_from_addresses_transition.output().copied();
                            let action_change_output = identity_create_from_addresses_action
                                .output()
                                .as_ref()
                                .copied();
                            assert_optional_action_output_state(
                                transition_change_output,
                                action_change_output,
                                &proof_address_infos_map,
                                "identity create from addresses change output",
                            );
                        } else {
                            // Verify the identity still does not exist since transition was not executed
                            let (root_hash, identity) = Drive::verify_full_identity_by_identity_id(
                                &response_proof.grovedb_proof,
                                false,
                                identity_create_from_addresses_action
                                    .identity_id()
                                    .into_buffer(),
                                platform_version,
                            )
                            .expect("expected to verify full identity");
                            assert_eq!(
                                &root_hash,
                                expected_root_hash,
                                "state last block info {:?}",
                                platform.state.last_committed_block_info()
                            );
                            assert!(identity.is_none());
                        }
                    } else {
                        panic!("expected identity create from addresses state transition");
                    }
                }
                StateTransitionAction::IdentityTopUpFromAddressesAction(
                    identity_top_up_from_addresses_action,
                ) => {
                    if let StateTransition::IdentityTopUpFromAddresses(
                        identity_top_up_from_addresses_transition,
                    ) = state_transition
                    {
                        let block_info = abci_app.platform.state.load().last_block_info().clone();
                        let (root_hash_identity, data) =
                            Drive::verify_state_transition_was_executed_with_proof(
                                state_transition,
                                &block_info,
                                response_proof.grovedb_proof.as_ref(),
                                &|_| Ok(None),
                                platform_version,
                            )
                            .expect("IdentityTopUpFromAddressesAction proof should verify");

                        let StateTransitionProofOutcome::AffectedState(
                            StateTransitionProofResult::VerifiedIdentityWithAddressInfos(
                                identity,
                                proof_address_infos,
                            ),
                        ) = data
                        else {
                            panic!("expected identity/address infos for top up from addresses proof, got {}",
                        std::any::type_name_of_val(&data));
                        };

                        assert!(
                            proof_address_infos.is_empty() == false,
                            "expected some address infos"
                        );
                        assert_eq!(
                            identity_top_up_from_addresses_action.identity_id(),
                            identity.id,
                            "expected identity ids to match"
                        );

                        // ensure all addresses used in the top-up are proved
                        let mut expected_addresses: BTreeSet<PlatformAddress> =
                            identity_top_up_from_addresses_transition
                                .inputs()
                                .keys()
                                .copied()
                                .collect();
                        if let Some((change_address, _)) =
                            identity_top_up_from_addresses_transition.output()
                        {
                            expected_addresses.insert(*change_address);
                        }
                        let proved_addresses: BTreeSet<PlatformAddress> =
                            proof_address_infos.keys().copied().collect();
                        assert_eq!(
                            proved_addresses, expected_addresses,
                            "proved addresses should match expected inputs and change"
                        );

                        assert_eq!(
                            &root_hash_identity,
                            expected_root_hash,
                            "state last block info {:?}",
                            platform.state.last_committed_block_info()
                        );
                        assert_address_inputs_state(
                            identity_top_up_from_addresses_transition.inputs(),
                            identity_top_up_from_addresses_action.inputs_with_remaining_balance(),
                            &proof_address_infos,
                            "identity top up from addresses",
                        );

                        let transition_change_output =
                            identity_top_up_from_addresses_transition.output().copied();
                        let action_change_output = identity_top_up_from_addresses_action.output();
                        assert_optional_action_output_state(
                            transition_change_output,
                            action_change_output,
                            &proof_address_infos,
                            "identity top up from addresses change output",
                        );
                    } else {
                        panic!("expected identity top up from addresses state transition");
                    }
                }
                StateTransitionAction::IdentityCreditTransferToAddressesAction(
                    identity_credit_transfer_to_addresses_action,
                ) => {
                    if let StateTransition::IdentityCreditTransferToAddresses(
                        identity_credit_transfer_to_addresses_transition,
                    ) = state_transition
                    {
                        let block_info = latest_block_info(&platform);

                        let (root_hash, proof_result) =
                            Drive::verify_state_transition_was_executed_with_proof(
                                state_transition,
                                &block_info,
                                &response_proof.grovedb_proof,
                                &|_| Ok(None),
                                platform_version,
                            )
                            .expect("expected to verify identity credit transfer proof");

                        assert_eq!(
                            &root_hash,
                            expected_root_hash,
                            "state last block info {:?}",
                            platform.state.last_committed_block_info()
                        );

                        if *was_executed {
                            let StateTransitionProofOutcome::AffectedState(
                                StateTransitionProofResult::VerifiedIdentityWithAddressInfos(
                                    proved_identity,
                                    proof_address_infos_map,
                                ),
                            ) = proof_result
                            else {
                                panic!("expected identity/address infos for credit transfer proof");
                            };

                            assert_eq!(
                                proved_identity.id,
                                identity_credit_transfer_to_addresses_action.identity_id()
                            );
                            assert_eq!(
                                proved_identity.id,
                                identity_credit_transfer_to_addresses_transition.identity_id()
                            );
                            assert!(
                                proved_identity.balance.is_some(),
                                "credit transfer proof should include updated identity balance"
                            );

                            let expected_addresses: BTreeSet<PlatformAddress> =
                                identity_credit_transfer_to_addresses_transition
                                    .recipient_addresses()
                                    .keys()
                                    .copied()
                                    .collect();

                            let proved_addresses: BTreeSet<PlatformAddress> =
                                proof_address_infos_map.keys().copied().collect();
                            assert_eq!(
                                proved_addresses, expected_addresses,
                                "proved addresses should match recipients"
                            );
                            assert!(
                                proof_address_infos_map
                                    .values()
                                    .all(|maybe_info| maybe_info.is_some()),
                                "all recipient addresses should return balance info"
                            );
                        }
                    } else {
                        panic!("expected identity credit transfer state transition");
                    }
                }
                StateTransitionAction::AddressFundsTransfer(_) => {
                    if let StateTransition::AddressFundsTransfer(_) = state_transition {
                        let block_info = latest_block_info(&platform);
                        assert_transition_yields_affected_state_snapshot(
                            state_transition,
                            &block_info,
                            &response_proof.grovedb_proof,
                            "address funds transfer",
                            platform_version,
                        );
                    } else {
                        panic!("expected address funds transfer state transition");
                    }
                }
                StateTransitionAction::AddressFundingFromAssetLock(
                    address_funding_from_asset_lock_action,
                ) => {
                    if let StateTransition::AddressFundingFromAssetLock(
                        address_funding_from_asset_lock_transition,
                    ) = state_transition
                    {
                        let block_info = latest_block_info(&platform);
                        assert_transition_yields_affected_state_snapshot(
                            state_transition,
                            &block_info,
                            &response_proof.grovedb_proof,
                            "address funding from asset lock",
                            platform_version,
                        );

                        let transition_inputs = address_funding_from_asset_lock_transition.inputs();
                        let transition_outputs =
                            address_funding_from_asset_lock_transition.outputs();
                        let action_outputs = address_funding_from_asset_lock_action.outputs();
                        let resolved_outputs =
                            address_funding_from_asset_lock_action.resolved_outputs();
                        tracing::debug!(
                            ?resolved_outputs,
                            "resolved outputs for address funding from asset lock"
                        );
                        assert_eq!(
                            transition_inputs,
                            address_funding_from_asset_lock_action.inputs_with_remaining_balance(),
                            "address funding from asset lock inputs should match the action"
                        );

                        assert_asset_lock_outputs_state(
                            transition_outputs,
                            action_outputs,
                            "address funding from asset lock outputs",
                        );
                    } else {
                        panic!("expected address funding from asset lock state transition");
                    }
                }
                StateTransitionAction::AddressCreditWithdrawal(_) => {
                    if let StateTransition::AddressCreditWithdrawal(_) = state_transition {
                        let block_info = latest_block_info(&platform);
                        assert_transition_yields_affected_state_snapshot(
                            state_transition,
                            &block_info,
                            &response_proof.grovedb_proof,
                            "address credit withdrawal",
                            platform_version,
                        );
                    } else {
                        panic!("expected address credit withdrawal state transition");
                    }
                }
                StateTransitionAction::BumpAddressInputNoncesAction(_) => {}
                StateTransitionAction::ShieldAction(_)
                | StateTransitionAction::ShieldedTransferAction(_)
                | StateTransitionAction::UnshieldAction(_)
                | StateTransitionAction::ShieldFromAssetLockAction(_)
                | StateTransitionAction::ShieldedWithdrawalAction(_)
                | StateTransitionAction::IdentityCreateFromShieldedPoolAction(_) => {
                    // The strategy harness does not generate shielded transitions (no shielded
                    // `OperationType`), so their proof-verification roundtrip isn't exercised here.
                    // IdentityCreateFromShieldedPool's strict prove/verify is covered by the unit
                    // test in rs-drive's verify module; its credit conservation is pinned by the
                    // converter conservation unit test (no AddToSystemCredits / RHS-internal).
                }
            }
        } else {
            // if we don't have an action this means there was a problem in the validation of the state transition
        }
    }

    log_state_transition_type_statistics();

    true
}
