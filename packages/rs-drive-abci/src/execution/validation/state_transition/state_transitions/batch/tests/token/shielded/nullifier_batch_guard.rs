//! The cap-two fixtures are deliberately synthetic: the funded pool is real, but the
//! spend note's client-tree anchor is inserted explicitly. Every spend carries a real
//! Validator-verified proof. Production batches remain capped at one.

use super::token_shielded_pool_tests::{
    assert_tokens_conserved, build_shield_bundle, build_spend_bundle, enable_shielded_pool,
    insert_token_pool_anchor, nullifier_is_spent, platform_with_latest_version, pool_balance,
    pool_notes_count, process, spendable_note, SHIELD_AMOUNT,
};
use super::*;
use crate::execution::check_tx::CheckTxLevel;
use crate::execution::validation::state_transition::processor::traits::identity_balance::StateTransitionIdentityBalanceValidationV0;
use crate::execution::types::execution_event::ExecutionEvent;
use crate::execution::types::state_transition_execution_context::{StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0};
use crate::execution::validation::state_transition::state_transitions::batch::state::v0::DocumentsBatchStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::state_transitions::batch::state::v0::fetch_documents::fetch_document_with_id;
use crate::execution::validation::state_transition::state_transitions::batch::transformer::v0::BatchTransitionTransformerV0;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::{PlatformRef, PlatformStateRef};
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::TempPlatform;
use dpp::data_contract::DataContract;
use dpp::data_contract::accessors::v0::DataContractV0Setters;
use dpp::data_contract::accessors::v1::DataContractV1Setters;
use dpp::data_contract::associated_token::token_configuration::accessors::v1::TokenConfigurationV1Setters;
use dpp::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::fee::fee_result::FeeResult;
use dpp::identity::{Identity, IdentityPublicKey};
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_nonce::IDENTITY_NONCE_VALUE_FILTER;
use dpp::shielded::{document_token_payment_extra_sighash_data_v0, token_shielded_transfer_extra_sighash_data_v0, OrchardBundleParams};
use dpp::shielded::token_burn_from_pool_extra_sighash_data_v0;
use dpp::state_transition::batch_transition::batched_transition::token_transition_action_type::TokenTransitionActionType;
use dpp::state_transition::batch_transition::BatchTransitionV1;
use dpp::state_transition::StateTransition;
use dpp::tokens::calculate_token_id;
use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
use dpp::tokens::token_payment_info::TokenPaymentInfo;
use dpp::tokens::token_payment_info::v0::TokenPaymentInfoV0;
use dpp::tests::json_document::json_document_to_json_value;
use dpp::system_data_contracts::dashpay_contract;
use dpp::validation::ConsensusValidationResult;
use dpp::util::hash::hash_single;
use dpp::version::DefaultForPlatformVersion;
use drive::drive::identity::key::fetch::IdentityKeysRequest;
use drive::drive::shielded::paths::{token_shielded_pool_path, SHIELDED_NOTES_KEY};
use drive::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use drive::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;
use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::token_transition::token_burn_from_pool_transition_action::TokenBurnFromPoolTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::token_transition::token_mint_to_pool_transition_action::TokenMintToPoolTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use drive::state_transition_action::StateTransitionAction;
use drive::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use drive::util::test_helpers::setup_contract;
use drive::util::storage_flags::StorageFlags;
use grovedb_commitment_tree::{FullViewingKey, SpendingKey};
use simple_signer::signer::SimpleSigner;

struct FundedPool {
    platform: TempPlatform<MockCoreRPCLike>,
    identity: Identity,
    signer: SimpleSigner,
    key: IdentityPublicKey,
    contract: DataContract,
    token_id: Identifier,
}

impl FundedPool {
    async fn sponsored_card(shielded: bool) -> Self {
        let version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let (identity, signer, key) = setup_identity(&mut platform, 70_501, dash_to_credits!(1.0));
        let (sponsor, _, _) = setup_identity(&mut platform, 70_507, dash_to_credits!(1.0));
        let contract_id = DataContract::generate_data_contract_id_v0(sponsor.id(), 1);
        let mut json = json_document_to_json_value(
            "tests/supporting_files/contract/crypto-card-game/crypto-card-game-in-game-currency-burn-tokens.json").unwrap();
        json["documentSchemas"]["card"]["tokenCost"]["create"]["gasFeesPaidBy"] =
            serde_json::json!(1);
        let mut contract = DataContract::from_json(json, true, version).unwrap();
        contract.set_id(contract_id);
        contract.set_owner_id(sponsor.id());
        contract.set_created_at_epoch(Some(0));
        contract.set_created_at(Some(0));
        contract.set_created_at_block_height(Some(0));
        contract
            .tokens_mut()
            .unwrap()
            .get_mut(&0)
            .unwrap()
            .set_has_shielded_pool(true);
        platform
            .drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                version,
            )
            .unwrap();
        let token_id = calculate_token_id(contract_id.as_bytes(), 0).into();
        super::document_shielded_token_payment_tests::fund_and_shield(
            &mut platform,
            &contract,
            token_id,
            &identity,
            &key,
            &signer,
            75,
            version,
        )
        .await;
        let balance = platform
            .drive
            .fetch_identity_balance(identity.id().to_buffer(), None, version)
            .unwrap()
            .unwrap();
        let retained = if shielded { 84_100_000 } else { 0 };
        platform
            .drive
            .remove_from_identity_balance(
                identity.id().to_buffer(),
                balance - retained,
                &BlockInfo::default(),
                true,
                None,
                version,
                None,
            )
            .unwrap();
        if !shielded {
            add_tokens_to_identity(&mut platform, token_id, identity.id(), 10);
        }
        Self {
            platform,
            identity,
            signer,
            key,
            contract,
            token_id,
        }
    }

    async fn group(mint: bool) -> Self {
        Self::group_with_required_power(mint, 2).await
    }

    async fn group_with_required_power(mint: bool, required_power: u32) -> Self {
        let version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let (identity, signer, key) = setup_identity(&mut platform, 70_501, dash_to_credits!(1.0));
        let (confirmer, _, _) = setup_identity(&mut platform, 70_504, dash_to_credits!(1.0));
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            Some(|configuration: &mut TokenConfiguration| {
                enable_shielded_pool(configuration);
                let rules = ChangeControlRules::V0(ChangeControlRulesV0 {
                    authorized_to_make_change: AuthorizedActionTakers::Group(0),
                    admin_action_takers: AuthorizedActionTakers::NoOne,
                    changing_authorized_action_takers_to_no_one_allowed: false,
                    changing_admin_action_takers_to_no_one_allowed: false,
                    self_changing_admin_action_takers_allowed: false,
                });
                if mint {
                    configuration.set_manual_minting_rules(rules);
                } else {
                    configuration.set_manual_burning_rules(rules);
                }
            }),
            None,
            Some(
                [(
                    0,
                    Group::V0(GroupV0 {
                        members: [(identity.id(), 1), (confirmer.id(), 1)].into(),
                        required_power,
                    }),
                )]
                .into(),
            ),
            None,
            version,
        );
        let shield = BatchTransition::new_token_shield_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            SHIELD_AMOUNT,
            build_shield_bundle(
                SHIELD_AMOUNT,
                75,
                TokenTransitionActionType::Shield,
                token_id,
                identity.id(),
            ),
            &key,
            2,
            0,
            &signer,
            version,
            None,
        )
        .await
        .unwrap();
        assert_matches!(
            process(&platform, &shield).execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        Self {
            platform,
            identity,
            signer,
            key,
            contract,
            token_id,
        }
    }

    async fn vouchers() -> Self {
        let version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let (identity, signer, key) = setup_identity(&mut platform, 70_501, dash_to_credits!(1.0));
        let contract_id = DataContract::generate_data_contract_id_v0(identity.id(), 1);
        let mut json = json_document_to_json_value(
            "tests/supporting_files/contract/crypto-card-game/crypto-card-game-in-game-currency-burn-tokens.json").unwrap();
        json["documentSchemas"] = serde_json::json!({
            "voucher": {
                "type": "object", "documentsMutable": false, "canBeDeleted": true, "transferable": 1,
                "properties": { "serial": { "type": "identifier", "position": 0 } },
                "indices": [{ "name": "serial", "properties": [{ "serial": "asc" }], "unique": true }],
                "required": ["serial", "$transferredAt", "$transferredAtBlockHeight", "$transferredAtCoreBlockHeight"], "additionalProperties": false,
                "tokenCost": { "transfer": { "tokenPosition": 0, "amount": 10, "effect": 1 } }
            },
            "redemption": {
                "type": "object", "documentsMutable": false, "canBeDeleted": false,
                "properties": { "voucherSerial": {
                    "type": "identifier", "position": 0,
                    "refersTo": { "type": "deletableDocument", "documentType": "voucher",
                        "findBy": { "serial": "." }, "where": { "$ownerId": "$ownerId" }, "consume": true }
                } },
                "required": ["voucherSerial"], "additionalProperties": false,
                "tokenCost": { "create": { "tokenPosition": 0, "amount": 10, "effect": 1 } }
            }
        });
        let mut contract =
            DataContract::from_json(json, true, version).expect("validated consuming contract");
        contract.set_id(contract_id);
        contract.set_owner_id(identity.id());
        contract.set_created_at_epoch(Some(0));
        contract.set_created_at(Some(0));
        contract.set_created_at_block_height(Some(0));
        contract
            .tokens_mut()
            .unwrap()
            .get_mut(&0)
            .unwrap()
            .set_has_shielded_pool(true);
        platform
            .drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                version,
            )
            .unwrap();
        let token_id = calculate_token_id(contract_id.as_bytes(), 0).into();
        super::document_shielded_token_payment_tests::fund_and_shield(
            &mut platform,
            &contract,
            token_id,
            &identity,
            &key,
            &signer,
            75,
            version,
        )
        .await;
        Self {
            platform,
            identity,
            signer,
            key,
            contract,
            token_id,
        }
    }

    async fn cards(burn: bool) -> Self {
        let version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let (identity, signer, key) = setup_identity(&mut platform, 70_501, dash_to_credits!(1.0));
        let contract_id = DataContract::generate_data_contract_id_v0(identity.id(), 1);
        let path = if burn {
            "tests/supporting_files/contract/crypto-card-game/crypto-card-game-in-game-currency-burn-tokens.json"
        } else {
            "tests/supporting_files/contract/crypto-card-game/crypto-card-game-in-game-currency.json"
        };
        let contract = setup_contract(
            &platform.drive,
            path,
            Some(contract_id.to_buffer()),
            Some(identity.id().to_buffer()),
            Some(|contract: &mut DataContract| {
                contract.set_created_at_epoch(Some(0));
                contract.set_created_at(Some(0));
                contract.set_created_at_block_height(Some(0));
                contract
                    .tokens_mut()
                    .unwrap()
                    .get_mut(&0)
                    .unwrap()
                    .set_has_shielded_pool(true);
            }),
            None,
            Some(version),
        );
        let token_id = calculate_token_id(contract_id.as_bytes(), 0).into();
        super::document_shielded_token_payment_tests::fund_and_shield(
            &mut platform,
            &contract,
            token_id,
            &identity,
            &key,
            &signer,
            75,
            version,
        )
        .await;
        Self {
            platform,
            identity,
            signer,
            key,
            contract,
            token_id,
        }
    }

    async fn new() -> Self {
        let version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        assert!(!platform.config.drive.batching_consistency_verification);
        let (identity, signer, key) = setup_identity(&mut platform, 70_501, dash_to_credits!(1.0));
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            Some(enable_shielded_pool),
            None,
            None,
            None,
            version,
        );
        let bundle = build_shield_bundle(
            SHIELD_AMOUNT,
            75,
            TokenTransitionActionType::Shield,
            token_id,
            identity.id(),
        );
        let shield = BatchTransition::new_token_shield_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            SHIELD_AMOUNT,
            bundle,
            &key,
            2,
            0,
            &signer,
            version,
            None,
        )
        .await
        .expect("shield transition");
        assert_matches!(
            process(&platform, &shield).execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        Self {
            platform,
            identity,
            signer,
            key,
            contract,
            token_id,
        }
    }

    async fn transfer(&self, bundle: OrchardBundleParams, nonce: u64) -> StateTransition {
        BatchTransition::new_token_shielded_transfer_transition(
            self.token_id,
            self.identity.id(),
            self.contract.id(),
            0,
            bundle,
            &self.key,
            nonce,
            0,
            &self.signer,
            PlatformVersion::latest(),
            None,
        )
        .await
        .expect("transfer transition")
    }

    fn transfers(&self, second_note: bool) -> (OrchardBundleParams, OrchardBundleParams, [u8; 32]) {
        let (note, anchor, path) = spendable_note(3_000, 81);
        insert_token_pool_anchor(&self.platform, self.token_id, &anchor);
        let fvk = FullViewingKey::from(&SpendingKey::from_bytes([7; 32]).unwrap());
        let nullifier = note.nullifier(&fvk).to_bytes();
        let extra = token_shielded_transfer_extra_sighash_data_v0(
            &self.token_id.to_buffer(),
            &self.identity.id().to_buffer(),
        );
        let (first, balance) = build_spend_bundle(note, path.clone(), anchor, 0, &extra, 76);
        assert_eq!(balance, 0);
        let (note, anchor, path) = if second_note {
            let fixture = spendable_note(3_000, 82);
            insert_token_pool_anchor(&self.platform, self.token_id, &fixture.1);
            fixture
        } else {
            (note, anchor, path)
        };
        let (second, balance) = build_spend_bundle(note, path, anchor, 0, &extra, 77);
        assert_eq!(balance, 0);
        assert!(first.actions.iter().any(|a| a.nullifier == nullifier));
        if !second_note {
            assert!(second.actions.iter().any(|a| a.nullifier == nullifier));
        }
        assert!(first
            .actions
            .iter()
            .all(|a| second.actions.iter().all(|b| a.cmx != b.cmx)));
        (first, second, nullifier)
    }

    async fn pair(&self, first: &StateTransition, second: &StateTransition) -> StateTransition {
        let StateTransition::Batch(BatchTransition::V1(first)) = first else {
            panic!("V1 batch")
        };
        let StateTransition::Batch(BatchTransition::V1(second)) = second else {
            panic!("V1 batch")
        };
        let mut transition: StateTransition = BatchTransitionV1 {
            owner_id: self.identity.id(),
            transitions: first
                .transitions
                .iter()
                .chain(&second.transitions)
                .cloned()
                .collect(),
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();
        transition
            .sign_external(
                &self.key,
                &self.signer,
                Some(|contract_id, document_type: String| {
                    assert_eq!(contract_id, self.contract.id());
                    Ok(self
                        .contract
                        .document_type_for_name(&document_type)?
                        .security_level_requirement())
                }),
            )
            .await
            .expect("aggregate signature");
        transition
    }

    fn validate(
        &self,
        transition: &StateTransition,
        version: &PlatformVersion,
    ) -> (
        ConsensusValidationResult<StateTransitionAction>,
        StateTransitionExecutionContext,
    ) {
        self.validate_order(transition, version, false)
    }

    fn validate_order(
        &self,
        transition: &StateTransition,
        version: &PlatformVersion,
        reverse: bool,
    ) -> (
        ConsensusValidationResult<StateTransitionAction>,
        StateTransitionExecutionContext,
    ) {
        self.validate_mode(
            transition,
            version,
            reverse,
            ValidationMode::Validator,
            false,
        )
    }

    fn validate_mode(
        &self,
        transition: &StateTransition,
        version: &PlatformVersion,
        reverse: bool,
        mode: ValidationMode,
        dry_run: bool,
    ) -> (
        ConsensusValidationResult<StateTransitionAction>,
        StateTransitionExecutionContext,
    ) {
        let StateTransition::Batch(batch) = transition else {
            panic!("batch")
        };
        let state = self.platform.state.load();
        let platform_ref = PlatformStateRef {
            drive: &self.platform.drive,
            state: &state,
            config: &self.platform.config,
        };
        let transaction = self.platform.drive.grove.start_transaction();
        let mut context =
            StateTransitionExecutionContext::default_for_platform_version(version).unwrap();
        if dry_run {
            context.enable_dry_run();
        }
        let transformed = batch
            .try_into_action_v0(
                &platform_ref,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                &mut context,
            )
            .expect("transform");
        assert!(
            transformed.is_valid(),
            "transform errors: {:?}",
            transformed.errors
        );
        let mut action = transformed.data.expect("action");
        // Transformation places documents first. Reversing the resolved vector exercises the
        // other validator order directly, without claiming network reachability for that order.
        if reverse {
            let mut transitions = action.transitions_take();
            transitions.reverse();
            action.set_transitions(transitions);
        }
        let result = batch
            .validate_state_v0(
                action,
                &platform_ref,
                &BlockInfo::default(),
                &mut context,
                mode,
                Some(&transaction),
                version,
            )
            .expect("state validation");
        (result, context)
    }

    fn execute(
        &self,
        transition: &StateTransition,
        validated: ConsensusValidationResult<StateTransitionAction>,
        context: StateTransitionExecutionContext,
    ) -> StateTransitionExecutionResult {
        let version = PlatformVersion::latest();
        let transaction = self.platform.drive.grove.start_transaction();
        let identity = self
            .platform
            .drive
            .fetch_identity_balance_with_keys(
                IdentityKeysRequest::new_specific_key_query(
                    &self.identity.id().to_buffer(),
                    self.key.id(),
                ),
                Some(&transaction),
                version,
            )
            .expect("load signing identity")
            .expect("identity exists");
        let (action, errors) = validated.into_data_and_errors().expect("validated action");
        let event = ExecutionEvent::create_from_state_transition_action(
            action,
            Some(identity),
            &BlockInfo::default().epoch,
            context,
            version,
        )
        .expect("execution event");
        let raw = transition.serialize_to_bytes().expect("serialize");
        let result = self
            .platform
            .platform
            .process_validation_result(
                &raw,
                "Batch",
                ConsensusValidationResult::new_with_data_and_errors(event, errors),
                &BlockInfo::default(),
                &transaction,
                &mut 0,
                version,
                &CachedEpochIndexFeeVersions::new(),
            )
            .expect("execute and charge real event");
        self.platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("commit");
        result
    }

    fn commitments(&self, start: u64) -> Vec<[u8; 32]> {
        self.platform
            .drive
            .grove
            .commitment_tree_get_range(
                token_shielded_pool_path(&self.token_id.to_buffer()).as_slice(),
                &[SHIELDED_NOTES_KEY],
                start,
                100,
                None,
                &PlatformVersion::latest().drive.grove_version,
            )
            .value
            .expect("landed note page")
            .entries
            .into_iter()
            .map(|(_, bytes)| bytes[..32].try_into().expect("stored commitment"))
            .collect()
    }
}

fn cap_two() -> PlatformVersion {
    let mut version = PlatformVersion::latest().clone();
    assert_eq!(version.system_limits.max_transitions_in_documents_batch, 1);
    version.system_limits.max_transitions_in_documents_batch = 2;
    version
}

#[tokio::test]
async fn should_preserve_single_entry_full_processor_fees_and_result() {
    let pool = FundedPool::new().await;
    let (bundle, _, _) = pool.transfers(false);
    let transfer = pool.transfer(bundle.clone(), 3).await;
    for (mode, dry_run) in [
        (ValidationMode::Validator, false),
        (ValidationMode::CheckTx, true),
        (ValidationMode::RecheckTx, true),
        (ValidationMode::NoValidation, true),
    ] {
        let (validated, context) =
            pool.validate_mode(&transfer, PlatformVersion::latest(), false, mode, dry_run);
        let operations = validated
            .data
            .clone()
            .expect("action")
            .into_high_level_drive_operations(
                &BlockInfo::default().epoch,
                PlatformVersion::latest(),
            )
            .unwrap();
        let snapshot = format!(
            "{:?}/{:?}/{:?}",
            validated,
            context.operations_slice(),
            operations
        );
        println!(
            "SINGLE ENTRY VALIDATION {mode:?}/{dry_run}: {}",
            hex::encode(hash_single(snapshot.as_bytes()))
        );
        assert_eq!(hex::encode(hash_single(snapshot.as_bytes())), "6762c961d9ebe20d958ed3c1fb4849e3e4e7e745a56843032136f98c414611db",
            "single-entry actions, errors, billed context and lowered operations must retain their characterization");
    }
    let before = pool_notes_count(&pool.platform, pool.token_id);
    let result = process(&pool.platform, &transfer);
    println!("UNCHANGED SINGLE ENTRY: {:?}", result.execution_results());
    assert_eq!(
        result.execution_results().as_slice(),
        &[StateTransitionExecutionResult::SuccessfulExecution {
            estimated_fees: Some(FeeResult {
                storage_fee: 40_230_000,
                processing_fee: 102_814_620,
                ..Default::default()
            }),
            fee_result: FeeResult {
                storage_fee: 28_674_000,
                processing_fee: 89_569_580,
                ..Default::default()
            },
            address_balance_changes: Default::default(),
        }]
    );
    assert_eq!(
        pool.commitments(before),
        bundle.actions.iter().map(|a| a.cmx).collect::<Vec<_>>()
    );
    assert_tokens_conserved(&pool.platform);
    println!(
        "SINGLE ENTRY ROOT: {}",
        hex::encode(
            pool.platform
                .drive
                .grove
                .root_hash(None, &PlatformVersion::latest().drive.grove_version)
                .value
                .unwrap()
        )
    );
    assert_eq!(
        hex::encode(
            pool.platform
                .drive
                .grove
                .root_hash(None, &PlatformVersion::latest().drive.grove_version)
                .value
                .unwrap()
        ),
        "c9acef9766a472f6d04abd174bf5efb0dcc22b107e7833156816dacefb7a2bc9"
    );
    assert_eq!(
        pool.platform
            .drive
            .fetch_identity_contract_nonce(
                pool.identity.id().to_buffer(),
                pool.contract.id().to_buffer(),
                true,
                None,
                PlatformVersion::latest()
            )
            .unwrap()
            .unwrap()
            & IDENTITY_NONCE_VALUE_FILTER,
        3
    );
}

#[tokio::test]
async fn should_reject_two_valid_spends_in_one_batch_when_test_cap_is_two() {
    let pool = FundedPool::new().await;
    let (first_bundle, second_bundle, nullifier) = pool.transfers(false);
    let first = pool.transfer(first_bundle.clone(), 3).await;
    let second = pool.transfer(second_bundle.clone(), 4).await;
    for candidate in [&first, &second] {
        let (validated, _) = pool.validate(candidate, PlatformVersion::latest());
        assert!(
            validated.is_valid(),
            "independent real proof must validate: {:?}",
            validated.errors
        );
    }
    let batch = pool.pair(&first, &second).await;
    let StateTransition::Batch(raw_batch) = &batch else {
        panic!("batch")
    };
    assert!(raw_batch
        .validate_base_structure(&cap_two())
        .unwrap()
        .is_valid());
    assert_matches!(
        raw_batch
            .validate_base_structure(PlatformVersion::latest())
            .unwrap()
            .errors
            .as_slice(),
        [ConsensusError::BasicError(
            BasicError::MaxDocumentsTransitionsExceededError(_)
        )]
    );
    let before = pool_notes_count(&pool.platform, pool.token_id);
    let (validated, context) = pool.validate(&batch, &cap_two());
    let errors = validated.errors.clone();
    let actions = validated.data.clone().expect("action");
    let result = pool.execute(&batch, validated, context);
    println!(
        "BATCH SAFETY OBSERVATION: errors={errors:?}, result={result:?}, appended={:?}",
        pool.commitments(before)
    );
    assert_eq!(
        pool_notes_count(&pool.platform, pool.token_id),
        before + first_bundle.actions.len() as u64,
        "only the first valid spend may append outputs"
    );
    assert_matches!(errors.as_slice(), [ConsensusError::StateError(StateError::NullifierAlreadySpentError(e))]
        if e.nullifier() == nullifier);
    let StateTransitionAction::BatchAction(action) = actions else {
        panic!("batch action")
    };
    assert_matches!(
        action.transitions().as_slice(),
        [
            BatchedTransitionAction::TokenAction(TokenTransitionAction::ShieldedTransferAction(_)),
            BatchedTransitionAction::BumpIdentityDataContractNonce(_)
        ]
    );
    assert_matches!(
        result,
        StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(StateError::NullifierAlreadySpentError(_)),
            ..
        }
    );
    assert_eq!(
        pool.commitments(before),
        first_bundle
            .actions
            .iter()
            .map(|a| a.cmx)
            .collect::<Vec<_>>()
    );
    for action in &second_bundle.actions {
        if action.nullifier != nullifier {
            assert!(!nullifier_is_spent(
                &pool.platform,
                pool.token_id,
                &action.nullifier
            ));
        }
    }
    assert_eq!(pool_balance(&pool.platform, pool.token_id), SHIELD_AMOUNT);
    assert_tokens_conserved(&pool.platform);
}

#[tokio::test]
async fn should_accept_distinct_real_spends_in_one_pool() {
    let pool = FundedPool::new().await;
    let (first_bundle, second_bundle, _) = pool.transfers(true);
    assert!(first_bundle.actions.iter().all(|a| second_bundle
        .actions
        .iter()
        .all(|b| a.nullifier != b.nullifier)));
    let first = pool.transfer(first_bundle.clone(), 3).await;
    let second = pool.transfer(second_bundle.clone(), 4).await;
    let batch = pool.pair(&first, &second).await;
    let before = pool_notes_count(&pool.platform, pool.token_id);
    let (validated, context) = pool.validate(&batch, &cap_two());
    assert!(
        validated.is_valid(),
        "distinct notes must not conflict: {:?}",
        validated.errors
    );
    assert_matches!(
        pool.execute(&batch, validated, context),
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    );
    let expected: Vec<_> = first_bundle
        .actions
        .iter()
        .chain(&second_bundle.actions)
        .map(|a| a.cmx)
        .collect();
    assert_eq!(pool.commitments(before), expected);
    assert_tokens_conserved(&pool.platform);
}

#[tokio::test]
async fn should_not_reserve_nullifiers_of_a_candidate_with_an_invalid_proof() {
    let pool = FundedPool::new().await;
    let (mut invalid_bundle, valid_bundle, _) = pool.transfers(false);
    invalid_bundle.proof[0] ^= 1;
    let first = pool.transfer(invalid_bundle, 3).await;
    let second = pool.transfer(valid_bundle.clone(), 4).await;
    let batch = pool.pair(&first, &second).await;
    let before = pool_notes_count(&pool.platform, pool.token_id);
    let (validated, context) = pool.validate(&batch, &cap_two());
    assert_matches!(
        validated.errors.as_slice(),
        [ConsensusError::StateError(
            StateError::InvalidShieldedProofError(_)
        )]
    );
    let Some(StateTransitionAction::BatchAction(action)) = &validated.data else {
        panic!("batch action")
    };
    assert_matches!(
        action.transitions().as_slice(),
        [
            BatchedTransitionAction::BumpIdentityDataContractNonce(_),
            BatchedTransitionAction::TokenAction(TokenTransitionAction::ShieldedTransferAction(_))
        ]
    );
    assert_matches!(
        pool.execute(&batch, validated, context),
        StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(StateError::InvalidShieldedProofError(_)),
            ..
        }
    );
    assert_eq!(
        pool.commitments(before),
        valid_bundle
            .actions
            .iter()
            .map(|a| a.cmx)
            .collect::<Vec<_>>()
    );
}

fn process_together(
    pool: &FundedPool,
    transitions: &[&StateTransition],
) -> Vec<StateTransitionExecutionResult> {
    let state = pool.platform.state.load();
    let raw: Vec<_> = transitions
        .iter()
        .map(|st| st.serialize_to_bytes().unwrap())
        .collect();
    let transaction = pool.platform.drive.grove.start_transaction();
    let result = pool
        .platform
        .platform
        .process_raw_state_transitions(
            &raw,
            &state,
            &BlockInfo::default(),
            &transaction,
            PlatformVersion::latest(),
            false,
            None,
        )
        .expect("full block processing");
    pool.platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("commit block");
    result.execution_results().clone()
}

#[tokio::test]
async fn should_reject_a_signed_two_token_batch_before_any_state_or_fee_effect() {
    let pool = FundedPool::new().await;
    let bundle = super::token_shielded_pool_tests::dummy_bundle();
    let first = pool.transfer(bundle.clone(), 3).await;
    let second = pool.transfer(bundle, 4).await;
    let batch = pool.pair(&first, &second).await;
    let state = pool.platform.state.load();
    let platform_ref = PlatformRef {
        drive: &pool.platform.drive,
        state: &state,
        config: &pool.platform.config,
        core_rpc: &pool.platform.core_rpc,
    };
    let raw = batch.serialize_to_bytes().unwrap();
    let root = pool
        .platform
        .drive
        .grove
        .root_hash(None, &PlatformVersion::latest().drive.grove_version)
        .value
        .unwrap();
    let check = pool
        .platform
        .check_tx(
            &raw,
            CheckTxLevel::FirstTimeCheck,
            &platform_ref,
            PlatformVersion::latest(),
        )
        .unwrap();
    assert_matches!(
        check.errors.as_slice(),
        [ConsensusError::BasicError(
            BasicError::MaxDocumentsTransitionsExceededError(_)
        )]
    );
    assert_matches!(
        process_together(&pool, &[&batch]).as_slice(),
        [StateTransitionExecutionResult::UnpaidConsensusError(
            ConsensusError::BasicError(BasicError::MaxDocumentsTransitionsExceededError(_))
        )]
    );
    assert_eq!(
        pool.platform
            .drive
            .grove
            .root_hash(None, &PlatformVersion::latest().drive.grove_version)
            .value
            .unwrap(),
        root,
        "cap rejection must charge nothing, bump no nonce and write no output/nullifier"
    );
}

#[tokio::test]
async fn should_see_the_first_real_spend_in_a_later_top_level_batch_of_the_same_block() {
    let pool = FundedPool::new().await;
    let (first_bundle, second_bundle, _) = pool.transfers(false);
    let first = pool.transfer(first_bundle.clone(), 3).await;
    let second = pool.transfer(second_bundle, 4).await;
    let before = pool_notes_count(&pool.platform, pool.token_id);
    assert_matches!(
        process_together(&pool, &[&first, &second]).as_slice(),
        [
            StateTransitionExecutionResult::SuccessfulExecution { .. },
            StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::NullifierAlreadySpentError(_)),
                ..
            }
        ]
    );
    assert_eq!(
        pool.commitments(before),
        first_bundle
            .actions
            .iter()
            .map(|a| a.cmx)
            .collect::<Vec<_>>()
    );
    assert_tokens_conserved(&pool.platform);
}

#[tokio::test]
async fn should_reject_an_outputs_only_bundle_repeated_in_one_batch() {
    let pool = FundedPool::new().await;
    let bundle = build_shield_bundle(
        100,
        84,
        TokenTransitionActionType::Shield,
        pool.token_id,
        pool.identity.id(),
    );
    let mut candidates = Vec::new();
    for nonce in [3, 4] {
        candidates.push(
            BatchTransition::new_token_shield_transition(
                pool.token_id,
                pool.identity.id(),
                pool.contract.id(),
                0,
                100,
                bundle.clone(),
                &pool.key,
                nonce,
                0,
                &pool.signer,
                PlatformVersion::latest(),
                None,
            )
            .await
            .unwrap(),
        );
    }
    let batch = pool.pair(&candidates[0], &candidates[1]).await;
    let before = pool_notes_count(&pool.platform, pool.token_id);
    let (validated, context) = pool.validate(&batch, &cap_two());
    assert_matches!(validated.errors.as_slice(), [ConsensusError::StateError(StateError::NullifierAlreadySpentError(e))]
        if e.nullifier() == bundle.actions[0].nullifier);
    assert_matches!(
        pool.execute(&batch, validated, context),
        StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(StateError::NullifierAlreadySpentError(_)),
            ..
        }
    );
    assert_eq!(
        pool.commitments(before),
        bundle.actions.iter().map(|a| a.cmx).collect::<Vec<_>>()
    );
    assert_eq!(
        pool_balance(&pool.platform, pool.token_id),
        SHIELD_AMOUNT + 100
    );
}

#[tokio::test]
async fn should_detect_a_transfer_and_document_payment_overlap_in_both_action_orders() {
    for burn in [false, true] {
        for transfer_first in [false, true] {
            let pool = FundedPool::cards(burn).await;
            let version = PlatformVersion::latest();
            let mut rng = StdRng::seed_from_u64(70_502);
            let card_type = pool.contract.document_type_for_name("card").unwrap();
            let (document, entropy) = super::document_shielded_token_payment_tests::random_card(
                &mut rng,
                card_type,
                pool.identity.id(),
                3,
                version,
            );
            let document_id = document.id();
            let (note, anchor, path) = spendable_note(15, 85);
            insert_token_pool_anchor(&pool.platform, pool.token_id, &anchor);
            let extra = token_shielded_transfer_extra_sighash_data_v0(
                &pool.token_id.to_buffer(),
                &pool.identity.id().to_buffer(),
            );
            let (transfer_bundle, _) =
                build_spend_bundle(note, path.clone(), anchor, 0, &extra, 86);
            let extra = document_token_payment_extra_sighash_data_v0(
                &pool.token_id.to_buffer(),
                &pool.identity.id().to_buffer(),
                &pool.contract.id().to_buffer(),
                &document_id.to_buffer(),
                10,
            );
            let (payment_bundle, _) = build_spend_bundle(note, path, anchor, 10, &extra, 87);
            let shared: Vec<_> = payment_bundle
                .actions
                .iter()
                .filter(|a| {
                    transfer_bundle
                        .actions
                        .iter()
                        .any(|b| b.nullifier == a.nullifier)
                })
                .collect();
            assert_eq!(shared.len(), 1);
            let nullifier = shared[0].nullifier;
            let payment = super::document_shielded_token_payment_tests::shielded_payment(
                payment_bundle.clone(),
                10,
            );
            let document_transition =
                BatchTransition::new_document_creation_transition_from_document(
                    document,
                    card_type,
                    entropy.0,
                    &pool.key,
                    3,
                    0,
                    Some(super::document_shielded_token_payment_tests::payment_info(
                        payment,
                    )),
                    &pool.signer,
                    version,
                    None,
                )
                .await
                .unwrap();
            let transfer = pool.transfer(transfer_bundle.clone(), 4).await;
            for candidate in [&document_transition, &transfer] {
                let (validated, _) = pool.validate(candidate, version);
                assert!(
                    validated.is_valid(),
                    "each separately proven candidate must validate: {:?}",
                    validated.errors
                );
            }
            let batch = pool.pair(&document_transition, &transfer).await;
            let before = pool_notes_count(&pool.platform, pool.token_id);
            let (validated, context) = pool.validate_order(&batch, &cap_two(), transfer_first);
            assert_matches!(validated.errors.as_slice(), [ConsensusError::StateError(StateError::NullifierAlreadySpentError(e))]
                if e.nullifier() == nullifier);
            let Some(StateTransitionAction::BatchAction(action)) = &validated.data else {
                panic!("batch action")
            };
            assert!(matches!(
                action.transitions()[1],
                BatchedTransitionAction::BumpIdentityDataContractNonce(_)
            ));
            assert_matches!(
                pool.execute(&batch, validated, context),
                StateTransitionExecutionResult::PaidConsensusError {
                    error: ConsensusError::StateError(StateError::NullifierAlreadySpentError(_)),
                    ..
                }
            );
            let expected = if transfer_first {
                &transfer_bundle
            } else {
                &payment_bundle
            };
            assert_eq!(
                pool.commitments(before),
                expected.actions.iter().map(|a| a.cmx).collect::<Vec<_>>()
            );
            assert_eq!(
                pool_balance(&pool.platform, pool.token_id),
                if transfer_first { 15 } else { 5 }
            );
            let found = fetch_document_with_id(
                &pool.platform.drive,
                &pool.contract,
                card_type,
                document_id,
                &BlockInfo::default().epoch,
                &mut StateTransitionExecutionContext::default_for_platform_version(version)
                    .unwrap(),
                None,
                version,
            )
            .unwrap();
            // State validation and event execution must also suppress the rejected document.
            assert_eq!(
                found.is_some(),
                !transfer_first,
                "a rejected payment must not create its document"
            );
            assert_tokens_conserved(&pool.platform);
        }
    }
}

#[tokio::test]
async fn should_not_reserve_a_payment_rejected_by_the_later_consumption_check() {
    let mut pool = FundedPool::vouchers().await;
    let version = PlatformVersion::latest();
    let (recipient, _, _) = setup_identity(&mut pool.platform, 70_506, dash_to_credits!(0.1));
    let mut rng = StdRng::seed_from_u64(70_503);
    let voucher_type = pool.contract.document_type_for_name("voucher").unwrap();
    let redemption_type = pool.contract.document_type_for_name("redemption").unwrap();
    let serial = [88; 32];
    let voucher_entropy = Bytes32::random_with_rng(&mut rng);
    let mut voucher = voucher_type
        .random_document_with_identifier_and_entropy(
            &mut rng,
            pool.identity.id(),
            voucher_entropy,
            DocumentFieldFillType::DoNotFillIfNotRequired,
            DocumentFieldFillSize::AnyDocumentFillSize,
            version,
        )
        .unwrap();
    voucher.set("serial", Value::Identifier(serial));
    voucher
        .set_id_for_creation(voucher_type, &voucher_entropy.0, 2, version)
        .unwrap();
    let create_voucher = BatchTransition::new_document_creation_transition_from_document(
        voucher.clone(),
        voucher_type,
        voucher_entropy.0,
        &pool.key,
        2,
        0,
        None,
        &pool.signer,
        version,
        None,
    )
    .await
    .unwrap();
    assert_matches!(
        process(&pool.platform, &create_voucher)
            .execution_results()
            .as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }]
    );
    let redemption_entropy = Bytes32::random_with_rng(&mut rng);
    let mut redemption = redemption_type
        .random_document_with_identifier_and_entropy(
            &mut rng,
            pool.identity.id(),
            redemption_entropy,
            DocumentFieldFillType::DoNotFillIfNotRequired,
            DocumentFieldFillSize::AnyDocumentFillSize,
            version,
        )
        .unwrap();
    redemption.set("voucherSerial", Value::Identifier(serial));
    redemption
        .set_id_for_creation(redemption_type, &redemption_entropy.0, 3, version)
        .unwrap();
    let redemption_id = redemption.id();
    let (note, anchor, path) = spendable_note(15, 89);
    insert_token_pool_anchor(&pool.platform, pool.token_id, &anchor);
    let extra = document_token_payment_extra_sighash_data_v0(
        &pool.token_id.to_buffer(),
        &pool.identity.id().to_buffer(),
        &pool.contract.id().to_buffer(),
        &redemption_id.to_buffer(),
        10,
    );
    let (create_bundle, _) = build_spend_bundle(note, path.clone(), anchor, 10, &extra, 90);
    let extra = document_token_payment_extra_sighash_data_v0(
        &pool.token_id.to_buffer(),
        &pool.identity.id().to_buffer(),
        &pool.contract.id().to_buffer(),
        &voucher.id().to_buffer(),
        10,
    );
    let (transfer_bundle, _) = build_spend_bundle(note, path, anchor, 10, &extra, 91);
    let create = BatchTransition::new_document_creation_transition_from_document(
        redemption,
        redemption_type,
        redemption_entropy.0,
        &pool.key,
        3,
        0,
        Some(super::document_shielded_token_payment_tests::payment_info(
            super::document_shielded_token_payment_tests::shielded_payment(create_bundle, 10),
        )),
        &pool.signer,
        version,
        None,
    )
    .await
    .unwrap();
    voucher.set_revision(Some(2));
    let transfer = BatchTransition::new_document_transfer_transition_from_document(
        voucher.clone(),
        voucher_type,
        recipient.id(),
        &pool.key,
        4,
        0,
        Some(super::document_shielded_token_payment_tests::payment_info(
            super::document_shielded_token_payment_tests::shielded_payment(
                transfer_bundle.clone(),
                10,
            ),
        )),
        &pool.signer,
        version,
        None,
    )
    .await
    .unwrap();
    for candidate in [&create, &transfer] {
        let (validated, _) = pool.validate(candidate, version);
        assert!(
            validated.is_valid(),
            "independently valid consuming/payment actions: {:?}",
            validated.errors
        );
    }
    let batch = pool.pair(&create, &transfer).await;
    let before = pool_notes_count(&pool.platform, pool.token_id);
    let (validated, context) = pool.validate(&batch, &cap_two());
    assert_matches!(
        validated.errors.as_slice(),
        [ConsensusError::StateError(
            StateError::ReferencedEntityNotFoundError(_)
        )]
    );
    let Some(StateTransitionAction::BatchAction(action)) = &validated.data else {
        panic!("batch action")
    };
    assert_matches!(
        action.transitions().as_slice(),
        [
            BatchedTransitionAction::BumpIdentityDataContractNonce(_),
            BatchedTransitionAction::DocumentAction(DocumentTransitionAction::TransferAction(_))
        ]
    );
    assert_matches!(
        pool.execute(&batch, validated, context),
        StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(_)),
            ..
        }
    );
    assert_eq!(
        pool.commitments(before),
        transfer_bundle
            .actions
            .iter()
            .map(|a| a.cmx)
            .collect::<Vec<_>>()
    );
    assert_eq!(pool_balance(&pool.platform, pool.token_id), 5);
    let mut context =
        StateTransitionExecutionContext::default_for_platform_version(version).unwrap();
    assert_eq!(
        fetch_document_with_id(
            &pool.platform.drive,
            &pool.contract,
            voucher_type,
            voucher.id(),
            &BlockInfo::default().epoch,
            &mut context,
            None,
            version
        )
        .unwrap()
        .unwrap()
        .owner_id(),
        recipient.id()
    );
    assert!(fetch_document_with_id(
        &pool.platform.drive,
        &pool.contract,
        redemption_type,
        redemption_id,
        &BlockInfo::default().epoch,
        &mut context,
        None,
        version
    )
    .unwrap()
    .is_none());
    assert_tokens_conserved(&pool.platform);
}

#[tokio::test]
async fn should_check_real_non_closing_mint_and_burn_groups_without_reserving_them() {
    for mint in [false, true] {
        for writer_first in [false, true] {
            let pool = FundedPool::group(mint).await;
            let version = PlatformVersion::latest();
            let (group_bundle, writer_bundle) = if mint {
                // Equal seeds produce equal dummy nullifiers, while the kind-bound signatures
                // are independently valid for mint and shield.
                (
                    build_shield_bundle(
                        100,
                        92,
                        TokenTransitionActionType::MintToPool,
                        pool.token_id,
                        pool.identity.id(),
                    ),
                    build_shield_bundle(
                        100,
                        92,
                        TokenTransitionActionType::Shield,
                        pool.token_id,
                        pool.identity.id(),
                    ),
                )
            } else {
                let (note, anchor, path) = spendable_note(3_000, 93);
                insert_token_pool_anchor(&pool.platform, pool.token_id, &anchor);
                let extra = token_burn_from_pool_extra_sighash_data_v0(
                    &pool.token_id.to_buffer(),
                    &pool.identity.id().to_buffer(),
                    100,
                );
                let (burn, _) = build_spend_bundle(note, path.clone(), anchor, 100, &extra, 94);
                let extra = token_shielded_transfer_extra_sighash_data_v0(
                    &pool.token_id.to_buffer(),
                    &pool.identity.id().to_buffer(),
                );
                let (transfer, _) = build_spend_bundle(note, path, anchor, 0, &extra, 95);
                (burn, transfer)
            };
            assert!(group_bundle.actions.iter().any(|a| writer_bundle
                .actions
                .iter()
                .any(|b| a.nullifier == b.nullifier)));
            let group_info =
                Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0));
            let group = if mint {
                BatchTransition::new_token_mint_to_pool_transition(
                    pool.token_id,
                    pool.identity.id(),
                    pool.contract.id(),
                    0,
                    100,
                    group_bundle,
                    None,
                    group_info,
                    &pool.key,
                    3,
                    0,
                    &pool.signer,
                    version,
                    None,
                )
                .await
                .unwrap()
            } else {
                BatchTransition::new_token_burn_from_pool_transition(
                    pool.token_id,
                    pool.identity.id(),
                    pool.contract.id(),
                    0,
                    100,
                    group_bundle,
                    None,
                    group_info,
                    &pool.key,
                    3,
                    0,
                    &pool.signer,
                    version,
                    None,
                )
                .await
                .unwrap()
            };
            let writer = if mint {
                BatchTransition::new_token_shield_transition(
                    pool.token_id,
                    pool.identity.id(),
                    pool.contract.id(),
                    0,
                    100,
                    writer_bundle.clone(),
                    &pool.key,
                    4,
                    0,
                    &pool.signer,
                    version,
                    None,
                )
                .await
                .unwrap()
            } else {
                pool.transfer(writer_bundle.clone(), 4).await
            };
            let (validated, _) = pool.validate(&group, version);
            assert!(
                validated.is_valid(),
                "real group candidate must validate: {:?}",
                validated.errors
            );
            let Some(StateTransitionAction::BatchAction(group_action)) = validated.data else {
                panic!("group batch")
            };
            match &group_action.transitions()[0] {
                BatchedTransitionAction::TokenAction(TokenTransitionAction::MintToPoolAction(
                    action,
                )) => assert!(!action.base().perform_action()),
                BatchedTransitionAction::TokenAction(
                    TokenTransitionAction::BurnFromPoolAction(action),
                ) => assert!(!action.base().perform_action()),
                other => panic!("non-closing pool group expected: {other:?}"),
            }
            assert!(pool.validate(&writer, version).0.is_valid());
            let batch = if writer_first {
                pool.pair(&writer, &group).await
            } else {
                pool.pair(&group, &writer).await
            };
            let before = pool_notes_count(&pool.platform, pool.token_id);
            let (validated, context) = pool.validate(&batch, &cap_two());
            if writer_first {
                assert_matches!(
                    validated.errors.as_slice(),
                    [ConsensusError::StateError(
                        StateError::NullifierAlreadySpentError(_)
                    )]
                );
            } else {
                assert!(
                    validated.is_valid(),
                    "a non-closing group reserves no pool nullifiers: {:?}",
                    validated.errors
                );
            }
            let result = pool.execute(&batch, validated, context);
            if writer_first {
                assert_matches!(
                    result,
                    StateTransitionExecutionResult::PaidConsensusError {
                        error: ConsensusError::StateError(StateError::NullifierAlreadySpentError(
                            _
                        )),
                        ..
                    }
                );
            } else {
                assert_matches!(
                    result,
                    StateTransitionExecutionResult::SuccessfulExecution { .. }
                );
            }
            assert_eq!(
                pool.commitments(before),
                writer_bundle
                    .actions
                    .iter()
                    .map(|a| a.cmx)
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                pool_balance(&pool.platform, pool.token_id),
                SHIELD_AMOUNT + if mint { 100 } else { 0 }
            );
            assert_tokens_conserved(&pool.platform);
        }
    }
}

#[tokio::test]
async fn should_reject_a_duplicate_payment_before_its_failing_trigger_without_charging_the_trigger()
{
    let mut pool = FundedPool::new().await;
    let version = PlatformVersion::latest();
    let mut json = json_document_to_json_value(
        "tests/supporting_files/contract/crypto-card-game/crypto-card-game-in-game-currency-burn-tokens.json").unwrap();
    json.as_object_mut().unwrap().remove("tokens");
    json["documentSchemas"] = serde_json::json!({ "contactRequest": {
        "type": "object", "documentsMutable": false, "canBeDeleted": false,
        "properties": { "toUserId": { "type": "identifier", "position": 0 } },
        "required": ["toUserId"], "additionalProperties": false,
        "tokenCost": { "create": { "contractId": pool.contract.id().to_buffer(),
            "tokenPosition": 0, "amount": 10, "effect": 0 } }
    } });
    let mut contract =
        DataContract::from_json(json, true, version).expect("valid trigger/payment contract");
    contract.set_id(dashpay_contract::ID.into());
    contract.set_owner_id(pool.identity.id());
    contract.set_created_at_epoch(Some(0));
    contract.set_created_at(Some(0));
    contract.set_created_at_block_height(Some(0));
    pool.platform
        .drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            version,
        )
        .unwrap();
    let document_type = contract.document_type_for_name("contactRequest").unwrap();
    let mut rng = StdRng::seed_from_u64(70_505);
    let entropy = Bytes32::random_with_rng(&mut rng);
    let mut document = document_type
        .random_document_with_identifier_and_entropy(
            &mut rng,
            pool.identity.id(),
            entropy,
            DocumentFieldFillType::DoNotFillIfNotRequired,
            DocumentFieldFillSize::AnyDocumentFillSize,
            version,
        )
        .unwrap();
    document.set("toUserId", Value::Identifier([96; 32]));
    document
        .set_id_for_creation(document_type, &entropy.0, 1, version)
        .unwrap();
    let document_id = document.id();
    let (note, anchor, path) = spendable_note(3_000, 97);
    insert_token_pool_anchor(&pool.platform, pool.token_id, &anchor);
    let extra = token_shielded_transfer_extra_sighash_data_v0(
        &pool.token_id.to_buffer(),
        &pool.identity.id().to_buffer(),
    );
    let (transfer_bundle, _) = build_spend_bundle(note, path.clone(), anchor, 0, &extra, 98);
    let extra = document_token_payment_extra_sighash_data_v0(
        &pool.token_id.to_buffer(),
        &pool.identity.id().to_buffer(),
        &contract.id().to_buffer(),
        &document_id.to_buffer(),
        10,
    );
    let (payment_bundle, _) = build_spend_bundle(note, path, anchor, 10, &extra, 99);
    let mut info = super::document_shielded_token_payment_tests::payment_info(
        super::document_shielded_token_payment_tests::shielded_payment(payment_bundle, 10),
    );
    let TokenPaymentInfo::V1(info_v1) = &mut info else {
        panic!("shielded payment")
    };
    info_v1.payment_token_contract_id = Some(pool.contract.id());
    let contact = BatchTransition::new_document_creation_transition_from_document(
        document,
        document_type,
        entropy.0,
        &pool.key,
        1,
        0,
        Some(info),
        &pool.signer,
        version,
        None,
    )
    .await
    .unwrap();
    let transfer = pool.transfer(transfer_bundle.clone(), 3).await;
    pool.platform.config.execution.use_document_triggers = false;
    assert!(
        pool.validate(&contact, version).0.is_valid(),
        "payment itself has a valid real proof"
    );
    pool.platform.config.execution.use_document_triggers = true;
    assert_matches!(
        pool.validate(&contact, version).0.errors.as_slice(),
        [ConsensusError::StateError(StateError::DataTriggerError(_))]
    );
    let batch = pool.pair(&contact, &transfer).await;
    let (natural, natural_context) = pool.validate_order(&batch, &cap_two(), false);
    assert_matches!(
        natural.errors.as_slice(),
        [ConsensusError::StateError(StateError::DataTriggerError(_))]
    );
    let Some(StateTransitionAction::BatchAction(action)) = &natural.data else {
        panic!("batch")
    };
    assert_matches!(
        action.transitions().as_slice(),
        [
            BatchedTransitionAction::BumpIdentityDataContractNonce(_),
            BatchedTransitionAction::TokenAction(TokenTransitionAction::ShieldedTransferAction(_))
        ]
    );
    let (duplicate, duplicate_context) = pool.validate_order(&batch, &cap_two(), true);
    assert_matches!(
        duplicate.errors.as_slice(),
        [ConsensusError::StateError(
            StateError::NullifierAlreadySpentError(_)
        )]
    );
    pool.platform.config.execution.use_document_triggers = false;
    let (_, disabled_natural_context) = pool.validate_order(&batch, &cap_two(), false);
    let (_, disabled_duplicate_context) = pool.validate_order(&batch, &cap_two(), true);
    pool.platform.config.execution.use_document_triggers = true;
    assert_eq!(
        natural_context.operations_slice().len(),
        disabled_natural_context.operations_slice().len() + 1,
        "the failing recipient trigger must actually perform and bill its identity lookup"
    );
    assert_eq!(
        format!("{:?}", duplicate_context.operations_slice()),
        format!("{:?}", disabled_duplicate_context.operations_slice()),
        "duplicate rejection must perform and bill no trigger work"
    );
    let before = pool_notes_count(&pool.platform, pool.token_id);
    assert_matches!(
        pool.execute(&batch, duplicate, duplicate_context),
        StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(StateError::NullifierAlreadySpentError(_)),
            ..
        }
    );
    assert_eq!(
        pool.commitments(before),
        transfer_bundle
            .actions
            .iter()
            .map(|a| a.cmx)
            .collect::<Vec<_>>()
    );
    assert!(fetch_document_with_id(
        &pool.platform.drive,
        &contract,
        document_type,
        document_id,
        &BlockInfo::default().epoch,
        &mut StateTransitionExecutionContext::default_for_platform_version(version).unwrap(),
        None,
        version
    )
    .unwrap()
    .is_none());
}

#[tokio::test]
async fn should_preserve_a_real_single_payment_sponsor_on_first_check_recheck_and_execution() {
    for shielded in [false, true] {
        let pool = FundedPool::sponsored_card(shielded).await;
        let version = PlatformVersion::latest();
        let mut rng = StdRng::seed_from_u64(70_508);
        let card_type = pool.contract.document_type_for_name("card").unwrap();
        let (document, entropy) = super::document_shielded_token_payment_tests::random_card(
            &mut rng,
            card_type,
            pool.identity.id(),
            2,
            version,
        );
        let (info, bundle) = if shielded {
            let (note, anchor, path) = spendable_note(15, 100);
            insert_token_pool_anchor(&pool.platform, pool.token_id, &anchor);
            let extra = document_token_payment_extra_sighash_data_v0(
                &pool.token_id.to_buffer(),
                &pool.identity.id().to_buffer(),
                &pool.contract.id().to_buffer(),
                &document.id().to_buffer(),
                10,
            );
            let (bundle, _) = build_spend_bundle(note, path, anchor, 10, &extra, 101);
            let mut info = super::document_shielded_token_payment_tests::payment_info(
                super::document_shielded_token_payment_tests::shielded_payment(bundle.clone(), 10),
            );
            let TokenPaymentInfo::V1(info_v1) = &mut info else {
                panic!("payment")
            };
            info_v1.gas_fees_paid_by = GasFeesPaidBy::ContractOwner;
            (info, Some(bundle))
        } else {
            (
                TokenPaymentInfo::V0(TokenPaymentInfoV0 {
                    payment_token_contract_id: None,
                    token_contract_position: 0,
                    minimum_token_cost: None,
                    maximum_token_cost: Some(10),
                    gas_fees_paid_by: GasFeesPaidBy::ContractOwner,
                }),
                None,
            )
        };
        let transition = BatchTransition::new_document_creation_transition_from_document(
            document,
            card_type,
            entropy.0,
            &pool.key,
            2,
            0,
            Some(info),
            &pool.signer,
            version,
            None,
        )
        .await
        .unwrap();
        let signing_identity = pool
            .platform
            .drive
            .fetch_identity_balance_with_keys(
                IdentityKeysRequest::new_specific_key_query(
                    &pool.identity.id().to_buffer(),
                    pool.key.id(),
                ),
                None,
                version,
            )
            .unwrap()
            .unwrap();
        assert_eq!(transition.relies_on_gas_sponsor_to_pay(&signing_identity, version).unwrap(), !shielded,
        "the current admission floor permits sponsored state-check routing only for the transparent payment");
        let state = pool.platform.state.load();
        let platform_ref = PlatformRef {
            drive: &pool.platform.drive,
            state: &state,
            config: &pool.platform.config,
            core_rpc: &pool.platform.core_rpc,
        };
        let raw = transition.serialize_to_bytes().unwrap();
        let root_before = pool
            .platform
            .drive
            .grove
            .root_hash(None, &version.drive.grove_version)
            .value
            .unwrap();
        for level in [CheckTxLevel::FirstTimeCheck, CheckTxLevel::Recheck] {
            let result = pool
                .platform
                .check_tx(&raw, level, &platform_ref, version)
                .unwrap();
            assert!(
                result.is_valid(),
                "real sponsored payment must pass {level:?}: {:?}",
                result.errors
            );
            assert_eq!(
                pool.platform
                    .drive
                    .grove
                    .root_hash(None, &version.drive.grove_version)
                    .value
                    .unwrap(),
                root_before
            );
        }
        let sponsor_before = pool
            .platform
            .drive
            .fetch_identity_balance(pool.contract.owner_id().to_buffer(), None, version)
            .unwrap()
            .unwrap();
        let before = pool_notes_count(&pool.platform, pool.token_id);
        let result = process(&pool.platform, &transition);
        println!(
            "SINGLE SPONSORED PAYMENT shielded={shielded}: {:?}",
            result.execution_results()
        );
        let (estimated_storage, estimated_processing, actual_storage, actual_processing) =
            if shielded {
                (171_126_000, 213_217_600, 155_142_000, 92_739_560)
            } else {
                (138_024_000, 117_541_680, 126_468_000, 3_870_440)
            };
        assert_eq!(
            result.execution_results().as_slice(),
            &[StateTransitionExecutionResult::SuccessfulExecution {
                estimated_fees: Some(FeeResult {
                    storage_fee: estimated_storage,
                    processing_fee: estimated_processing,
                    ..Default::default()
                }),
                fee_result: FeeResult {
                    storage_fee: actual_storage,
                    processing_fee: actual_processing,
                    ..Default::default()
                },
                address_balance_changes: Default::default(),
            }]
        );
        let [StateTransitionExecutionResult::SuccessfulExecution { fee_result, .. }] =
            result.execution_results().as_slice()
        else {
            panic!(
                "sponsored payment must execute: {:?}",
                result.execution_results()
            );
        };
        assert_eq!(
            pool.platform
                .drive
                .fetch_identity_balance(pool.identity.id().to_buffer(), None, version)
                .unwrap(),
            Some(if shielded { 84_100_000 } else { 0 })
        );
        assert_eq!(
            sponsor_before
                - pool
                    .platform
                    .drive
                    .fetch_identity_balance(pool.contract.owner_id().to_buffer(), None, version)
                    .unwrap()
                    .unwrap(),
            fee_result.processing_fee + fee_result.storage_fee
        );
        if let Some(bundle) = bundle {
            assert_eq!(
                pool.commitments(before),
                bundle.actions.iter().map(|a| a.cmx).collect::<Vec<_>>()
            );
            assert_eq!(pool_balance(&pool.platform, pool.token_id), 5);
        } else {
            assert_eq!(pool_notes_count(&pool.platform, pool.token_id), before);
            assert_eq!(pool_balance(&pool.platform, pool.token_id), 15);
        }
        assert_tokens_conserved(&pool.platform);
    }
}

#[tokio::test]
async fn should_preserve_single_entry_actions_errors_operations_and_settled_state() {
    let version = PlatformVersion::latest();
    for case in [
        "shield",
        "valid-payment",
        "failed-payment",
        "within-bundle-repeat",
        "stored-repeat",
        "open-mint-group",
        "closing-mint-group",
        "open-burn-group",
        "closing-burn-group",
    ] {
        let pool = if case.ends_with("payment") {
            FundedPool::cards(true).await
        } else if case.contains("group") {
            FundedPool::group_with_required_power(
                case.contains("mint"),
                if case.starts_with("open") { 2 } else { 1 },
            )
            .await
        } else {
            FundedPool::new().await
        };
        let transition = if case == "shield" {
            BatchTransition::new_token_shield_transition(
                pool.token_id,
                pool.identity.id(),
                pool.contract.id(),
                0,
                100,
                build_shield_bundle(
                    100,
                    102,
                    TokenTransitionActionType::Shield,
                    pool.token_id,
                    pool.identity.id(),
                ),
                &pool.key,
                3,
                0,
                &pool.signer,
                version,
                None,
            )
            .await
            .unwrap()
        } else if case.ends_with("payment") {
            let mut rng = StdRng::seed_from_u64(70_509);
            let document_type = pool.contract.document_type_for_name("card").unwrap();
            let (document, entropy) = super::document_shielded_token_payment_tests::random_card(
                &mut rng,
                document_type,
                pool.identity.id(),
                2,
                version,
            );
            let (note, anchor, path) = spendable_note(15, 103);
            insert_token_pool_anchor(&pool.platform, pool.token_id, &anchor);
            let extra = document_token_payment_extra_sighash_data_v0(
                &pool.token_id.to_buffer(),
                &pool.identity.id().to_buffer(),
                &pool.contract.id().to_buffer(),
                &document.id().to_buffer(),
                10,
            );
            let (mut bundle, _) = build_spend_bundle(note, path, anchor, 10, &extra, 104);
            if case == "failed-payment" {
                bundle.proof[0] ^= 1;
            }
            BatchTransition::new_document_creation_transition_from_document(
                document,
                document_type,
                entropy.0,
                &pool.key,
                2,
                0,
                Some(super::document_shielded_token_payment_tests::payment_info(
                    super::document_shielded_token_payment_tests::shielded_payment(bundle, 10),
                )),
                &pool.signer,
                version,
                None,
            )
            .await
            .unwrap()
        } else if case.contains("group") {
            let bundle = if case.contains("mint") {
                build_shield_bundle(
                    100,
                    105,
                    TokenTransitionActionType::MintToPool,
                    pool.token_id,
                    pool.identity.id(),
                )
            } else {
                let (note, anchor, path) = spendable_note(3_000, 106);
                insert_token_pool_anchor(&pool.platform, pool.token_id, &anchor);
                let extra = token_burn_from_pool_extra_sighash_data_v0(
                    &pool.token_id.to_buffer(),
                    &pool.identity.id().to_buffer(),
                    100,
                );
                build_spend_bundle(note, path, anchor, 100, &extra, 107).0
            };
            if case.contains("mint") {
                BatchTransition::new_token_mint_to_pool_transition(
                    pool.token_id,
                    pool.identity.id(),
                    pool.contract.id(),
                    0,
                    100,
                    bundle,
                    None,
                    Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
                    &pool.key,
                    3,
                    0,
                    &pool.signer,
                    version,
                    None,
                )
                .await
                .unwrap()
            } else {
                BatchTransition::new_token_burn_from_pool_transition(
                    pool.token_id,
                    pool.identity.id(),
                    pool.contract.id(),
                    0,
                    100,
                    bundle,
                    None,
                    Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
                    &pool.key,
                    3,
                    0,
                    &pool.signer,
                    version,
                    None,
                )
                .await
                .unwrap()
            }
        } else {
            let (mut bundle, _, _) = pool.transfers(false);
            if case == "within-bundle-repeat" {
                bundle.actions[1].nullifier = bundle.actions[0].nullifier;
            }
            let nonce = if case == "stored-repeat" {
                assert_matches!(
                    process(&pool.platform, &pool.transfer(bundle.clone(), 3).await)
                        .execution_results()
                        .as_slice(),
                    [StateTransitionExecutionResult::SuccessfulExecution { .. }]
                );
                4
            } else {
                3
            };
            pool.transfer(bundle, nonce).await
        };
        let root_before = pool
            .platform
            .drive
            .grove
            .root_hash(None, &version.drive.grove_version)
            .value
            .unwrap();
        let (validated, context) = pool.validate(&transition, version);
        match case {
            "failed-payment" => assert_matches!(
                validated.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::InvalidShieldedProofError(_)
                )]
            ),
            "within-bundle-repeat" | "stored-repeat" => assert_matches!(
                validated.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::NullifierAlreadySpentError(_)
                )]
            ),
            _ => assert!(validated.is_valid(), "{case}: {:?}", validated.errors),
        }
        let operations = validated
            .data
            .clone()
            .unwrap()
            .into_high_level_drive_operations(&BlockInfo::default().epoch, version)
            .unwrap();
        let validation = format!(
            "{validated:?}/{:?}/{operations:?}",
            context.operations_slice()
        );
        let result = process(&pool.platform, &transition);
        let root_after = pool
            .platform
            .drive
            .grove
            .root_hash(None, &version.drive.grove_version)
            .value
            .unwrap();
        let nonce = pool
            .platform
            .drive
            .fetch_identity_contract_nonce(
                pool.identity.id().to_buffer(),
                pool.contract.id().to_buffer(),
                true,
                None,
                version,
            )
            .unwrap();
        let settled = format!(
            "{root_before:?}/{:?}/{root_after:?}/{nonce:?}",
            result.execution_results()
        );
        println!(
            "SINGLE ENTRY PARITY {case}: validation={}, settled={}, result={:?}",
            hex::encode(hash_single(validation.as_bytes())),
            hex::encode(hash_single(settled.as_bytes())),
            result.execution_results()
        );
        let (expected_validation, expected_settled) = match case {
            "shield" => (
                "7e3cf25f4080c94ef028ede8d3bdc982d0055feaef0d9e75dcae11287146d425",
                "a92938b72bd217bbfcb2eae4543cc1f0952603846996138137171f8b09ee3a6f",
            ),
            "valid-payment" => (
                "da69c2e10a0fe88df56655907ae5c3acd6700f13823358159f6240b1fb33c6d1",
                "bf6c31ef6b600d6967970f8281f31cab6f2458c278af026aeca653402f01beb0",
            ),
            "failed-payment" => (
                "d9b5fe191841bea952494f2a45e3833f925f4d367a1937b2ef1235a1aa01f51e",
                "49c5e6beeec4a7a5a965c879db6741fa0b06984838d320737696b2b38e02d3a5",
            ),
            "within-bundle-repeat" => (
                "8023e03a697a20f6a78396a05503a4bc583ca8f69e19bd7ee07a6ae0ce7be478",
                "5e4cdb13a64c4aa4021db2b1f8b904802ad5e2da59b8a134a22a3f0f65b0ab8e",
            ),
            "stored-repeat" => (
                "05652ac85c972902e8c09d20a7d8586e17e9434a478b335da136ad85f73def01",
                "67edd91cc0dd89e5311e2fb8cf44674dce2297f54355812596fe9e7e44f18deb",
            ),
            "open-mint-group" => (
                "7f694d1f3507aaca0c5b5b1254eef3f8cd595379c23c32dea1d0a9c21d302088",
                "3e4998d3dae6cfd9f92cb2423b3ff487916d824a5ab4a633410a951eccf4a6cc",
            ),
            "closing-mint-group" => (
                "1901253314ac7c543ffa24ae22b91f6790365b66bfa9b477dce90bacd3c97e75",
                "842e80ea186d371ef328afae81ef71489e78d18f42b7e90d022b58a29c4f7572",
            ),
            "open-burn-group" => (
                "eb4786ab90fb36ffc33d8167e199316929ef28cceb0dcff61e6612cd6fcc1d73",
                "da0199ba62b4091a20128874f0246d55066214078081ebd282ab97acf400e6b8",
            ),
            "closing-burn-group" => (
                "dbb52b37ff8c4b451cf36ee3a6a8b1d299d0f786f55cad6c700ef1b947bc1179",
                "dd6061541bca7a1f1e6137f5418bf49b8e4ff40b3fc076817511727327670e11",
            ),
            _ => unreachable!("enumerated characterization case"),
        };
        assert_eq!(hex::encode(hash_single(validation.as_bytes())), expected_validation,
            "{case}: actions, errors, context charges and lowered operations must retain the original characterization");
        assert_eq!(hex::encode(hash_single(settled.as_bytes())), expected_settled,
            "{case}: full processor results, exact fees, root and nonce must retain the original characterization");
        assert_tokens_conserved(&pool.platform);
    }
}
